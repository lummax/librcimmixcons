// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod block_info;
mod block_allocator;
mod allocator;

use self::block_allocator::BlockAllocator;
use self::allocator::Allocator;
use self::allocator::NormalAllocator;
use self::allocator::OverflowAllocator;
use self::allocator::EvacAllocator;

pub use self::block_info::BlockInfo;

use std::{mem, ptr};
use std::collections::RingBuf;
use std::rc::Rc;
use std::cell::RefCell;

use constants::{BLOCK_SIZE, MEDIUM_OBJECT};
use gc_object::{GCRTTI, GCObject, GCObjectRef};

/// The `ImmixSpace` is the default space to allocate objects into.
///
/// Objects larger than `LARGE_OBJECT` are allocated using the
/// `LargeObjectSpace`.
///
/// The `ImmixSpace` partitions the heap into `BLOCK_SIZE` blocks of
/// `LINE_SIZE` lines. Objects are allocated into free lines on free or
/// partially used blocks.
///
/// Lines are marked with the number of live objects. This counter is
/// maintained by the `RCCollector` and the `ImmixCollector`. If it drops to
/// zero the line can be reclaimed. If a block has only free lines it can be
/// returned to the global block allocator.
pub struct ImmixSpace {
    /// The global `BlockAllocator` to get new blocks from.
    block_allocator: Rc<RefCell<BlockAllocator>>,

    /// The nomal allocator for objects smaller than `MEDIUM_OBJECT` bytes.
    allocator: NormalAllocator,

    /// The overflow allocator for objects larger than `MEDIUM_OBJECT` bytes.
    overflow_allocator: OverflowAllocator,

    /// The evacuation allocator used during an evacuating collection.
    evac_allocator: EvacAllocator,

    /// The current live mark for new objects. See `Spaces.current_live_mark`.
    current_live_mark: bool,
}

impl ImmixSpace {
    /// Create a new `ImmixSpace`.
    ///
    /// This also initializes the `BlockAllocator` which will allocate a
    /// memory map of `HEAP_SIZE` bytes. The allocation will fail if there is
    /// not enough memory available.
    pub fn new() -> ImmixSpace {
        let block_allocator = Rc::new(RefCell::new(BlockAllocator::new()));
        let normal_block_allocator = block_allocator.clone();
        let overflow_block_allocator = block_allocator.clone();
        return ImmixSpace {
            block_allocator: block_allocator,
            allocator: NormalAllocator::new(normal_block_allocator),
            overflow_allocator: OverflowAllocator::new(overflow_block_allocator),
            evac_allocator: EvacAllocator::new(),
            current_live_mark: false,
        };
    }

    /// Decrement the lines on which the object is allocated.
    pub fn decrement_lines(&self, object: GCObjectRef) {
        debug_assert!(self.is_gc_object(object),
                     "decrement_lines() on invalid object {:p}", object);
        unsafe{ (*ImmixSpace::get_block_ptr(object)).decrement_lines(object); }
    }

    /// Increment the lines on which the object is allocated.
    pub fn increment_lines(&self, object: GCObjectRef) {
        debug_assert!(self.is_gc_object(object),
                      "increment_lines() on invalid object {:p}", object);
        unsafe{ (*ImmixSpace::get_block_ptr(object)).increment_lines(object); }
    }

    /// Set an address in this space as a valid object.
    pub fn set_gc_object(&self, object: GCObjectRef) {
        debug_assert!(self.block_allocator.borrow().is_in_space(object),
                      "set_gc_object() on invalid object {:p}", object);
        unsafe{ (*ImmixSpace::get_block_ptr(object)).set_gc_object(object); }
    }

    /// Unset an address as a valid object within the immix space.
    pub fn unset_gc_object(&self, object: GCObjectRef) {
        debug_assert!(self.block_allocator.borrow().is_in_space(object),
                      "unset_gc_object() on invalid object {:p}", object);
        unsafe{ (*ImmixSpace::get_block_ptr(object)).unset_gc_object(object); }
    }

    /// Return if the object an the address is a valid object within the immix
    /// space.
    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        if self.block_allocator.borrow().is_in_space(object) {
            return unsafe{ (*ImmixSpace::get_block_ptr(object)).is_gc_object(object) };
        }
        return false;
    }

    /// Return the total number of possible blocks.
    pub fn total_blocks(&self) -> usize {
        return self.block_allocator.borrow().total_blocks();
    }

    /// Return the number of unallocated blocks.
    pub fn available_blocks(&self) -> usize {
        return self.block_allocator.borrow().available_blocks();
    }

    /// Get the number of currently free blocks in the evacuation allocator.
    pub fn evac_headroom(&self) -> usize {
        return self.evac_allocator.evac_headroom();
    }

    /// Return a collection of blocks to the global block allocator.
    pub fn return_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.block_allocator.borrow_mut().return_blocks(blocks);
    }

    /// Set the current live mark to `current_live_mark`.
    pub fn set_current_live_mark(&mut self, current_live_mark: bool) {
        self.current_live_mark = current_live_mark;
    }

    /// Set the recyclable blocks for the `NormalAllocator`.
    pub fn set_recyclable_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.allocator.set_recyclable_blocks(blocks);
    }

    /// Extend the list of free blocks in the `EvacAllocator` for evacuation.
    pub fn extend_evac_headroom(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.evac_allocator.extend_evac_headroom(blocks);
    }

    /// Get all block managed by all allocators, draining any local
    /// collections.
    pub fn get_all_blocks(&mut self) -> RingBuf<*mut BlockInfo> {
        let mut normal_blocks = self.allocator.get_all_blocks();
        let mut overflow_blocks = self.overflow_allocator.get_all_blocks();
        let mut evac_blocks = self.evac_allocator.get_all_blocks();
        return normal_blocks.drain()
                            .chain(overflow_blocks.drain())
                            .chain(evac_blocks.drain())
                            .collect();
    }

    /// Allocate an object of `size` bytes or return `None` if the allocation
    /// failed.
    ///
    /// This object is initialized and ready to use.
    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        if let Some(object) = if size < MEDIUM_OBJECT { self.allocator.allocate(size) }
                              else { self.overflow_allocator.allocate(size) } {
            unsafe { ptr::write(object, GCObject::new(rtti, self.current_live_mark)); }
            unsafe{ (*ImmixSpace::get_block_ptr(object)).set_new_object(object); }
            self.set_gc_object(object);
            return Some(object);
        }
        return None;
    }

    /// Evacuate the object to another block using the `EvacAllocator`
    /// returning the new address or `None` if no evacuation was performed.
    ///
    /// An object is evacuated if it is not pinned, it resides on an
    /// evacuation candidate block and the evacuation allocator hat enough
    /// space left.
    ///
    /// On successful evacuation the old object is marked as forewarded an an
    /// forewarding pointer is installed.
    pub fn maybe_evacuate(&mut self, object: GCObjectRef) -> Option<GCObjectRef> {
        let block_info = unsafe{ ImmixSpace::get_block_ptr(object) };
        let is_pinned = unsafe{ (*object).is_pinned() };
        let is_candidate = unsafe{ (*block_info).is_evacuation_candidate() };
        if is_pinned || !is_candidate {
            return None;
        }
        let size = unsafe{ (*object).object_size() };
        if let Some(new_object) = self.evac_allocator.allocate(size) {
            unsafe{
                ptr::copy_nonoverlapping_memory(new_object as *mut u8,
                                                object as *const u8, size);
                debug_assert!(*object == *new_object,
                              "Evacuated object was not copied correcty");
                (*object).set_forwarded(new_object);
                self.set_gc_object(object);
            }
            debug!("Evacuated object {:p} from block {:p} to {:p}", object,
                   block_info, new_object);
            valgrind_freelike!(object);
            return Some(new_object);
        }
        debug!("Can't evacuation object {:p} from block {:p}", object, block_info);
        return None;
    }
}

impl ImmixSpace {
    /// Get the block for the given object.
    unsafe fn get_block_ptr(object: GCObjectRef) -> *mut BlockInfo {
        let block_offset = object as usize % BLOCK_SIZE;
        let block = mem::transmute((object as *mut u8).offset(-(block_offset as isize)));
        debug!("Block for object {:p}: {:p} with offset: {}", object, block, block_offset);
        return block;
    }
}
