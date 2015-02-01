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

pub struct ImmixSpace {
    block_allocator: Rc<RefCell<BlockAllocator>>,
    allocator: NormalAllocator,
    overflow_allocator: OverflowAllocator,
    evac_allocator: EvacAllocator,
    current_live_mark: bool,
}

impl ImmixSpace {
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

    pub fn decrement_lines(object: GCObjectRef) {
        unsafe{ (*ImmixSpace::get_block_ptr(object)).decrement_lines(object); }
    }

    pub fn increment_lines(object: GCObjectRef) {
        unsafe{ (*ImmixSpace::get_block_ptr(object)).increment_lines(object); }
    }

    pub fn set_gc_object(object: GCObjectRef) {
        unsafe{ (*ImmixSpace::get_block_ptr(object)).set_gc_object(object); }
    }

    pub fn unset_gc_object(object: GCObjectRef) {
        unsafe{ (*ImmixSpace::get_block_ptr(object)).unset_gc_object(object); }
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        if self.block_allocator.borrow().is_in_space(object) {
            return unsafe{ (*ImmixSpace::get_block_ptr(object)).is_gc_object(object) };
        }
        return false;
    }

    pub fn total_blocks(&self) -> usize {
        return self.block_allocator.borrow().total_blocks();
    }

    pub fn available_blocks(&self) -> usize {
        return self.block_allocator.borrow().available_blocks();
    }

    pub fn evac_headroom(&self) -> usize {
        return self.evac_allocator.evac_headroom();
    }

    pub fn return_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.block_allocator.borrow_mut().return_blocks(blocks);
    }

    pub fn set_current_live_mark(&mut self, current_live_mark: bool) {
        self.current_live_mark = current_live_mark;
    }

    pub fn set_recyclable_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.allocator.set_recyclable_blocks(blocks);
    }

    pub fn extend_evac_headroom(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.evac_allocator.extend_evac_headroom(blocks);
    }

    pub fn get_all_blocks(&mut self) -> RingBuf<*mut BlockInfo> {
        return self.allocator.get_all_blocks().drain()
                   .chain(self.overflow_allocator.get_all_blocks().drain())
                   .chain(self.evac_allocator.get_all_blocks().drain())
                   .collect();
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        if let Some(object) = if size < MEDIUM_OBJECT { self.allocator.allocate(size) }
                              else { self.overflow_allocator.allocate(size) } {
            unsafe { ptr::write(object, GCObject::new(rtti, self.current_live_mark)); }
            unsafe{ (*ImmixSpace::get_block_ptr(object)).set_new_object(object); }
            ImmixSpace::set_gc_object(object);
            return Some(object);
        }
        return None;
    }

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
                ImmixSpace::unset_gc_object(object);
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
    unsafe fn get_block_ptr(object: GCObjectRef) -> *mut BlockInfo {
        let block_offset = object as usize % BLOCK_SIZE;
        let block = mem::transmute((object as *mut u8).offset(-(block_offset as isize)));
        debug!("Block for object {:p}: {:p} with offset: {}", object, block, block_offset);
        return block;
    }
}
