// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod block_info;
mod block_allocator;
mod allocator;
mod collector;

use self::block_info::BlockInfo;
use self::block_allocator::BlockAllocator;
use self::allocator::Allocator;
use self::allocator::NormalAllocator;
use self::allocator::OverflowAllocator;
use self::allocator::EvacAllocator;
use self::collector::Collector;

use std::{mem, ptr};
use std::rc::Rc;
use std::cell::RefCell;

use constants::{BLOCK_SIZE, LINE_SIZE};
use gc_object::{GCRTTI, GCObject, GCObjectRef};
use spaces::CollectionType;

pub struct ImmixSpace {
    block_allocator: Rc<RefCell<BlockAllocator>>,
    allocator: NormalAllocator,
    overflow_allocator: OverflowAllocator,
    evac_allocator: EvacAllocator,
    collector: Collector,
    current_live_mark: bool,
}

impl ImmixSpace {
    pub fn new() -> ImmixSpace {
        let block_allocator = Rc::new(RefCell::new(BlockAllocator::new()));
        let normal_block_allocator = block_allocator.clone();
        let overflow_block_allocator = block_allocator.clone();
        let collector_block_allocator = block_allocator.clone();
        return ImmixSpace {
            block_allocator: block_allocator,
            allocator: NormalAllocator::new(normal_block_allocator),
            overflow_allocator: OverflowAllocator::new(overflow_block_allocator),
            evac_allocator: EvacAllocator::new(),
            collector: Collector::new(collector_block_allocator),
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
        if self.is_in_space(object) {
            return unsafe{ (*ImmixSpace::get_block_ptr(object)).is_gc_object(object) };
        }
        return false;
    }

    pub fn is_in_space(&self, object: GCObjectRef) -> bool {
        return self.block_allocator.borrow().is_in_space(object);
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        if self.is_gc_object(object) {
            self.collector.write_barrier(object);
        }
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        if let Some(object) = if size < LINE_SIZE { self.allocator.allocate(size) }
                              else { self.overflow_allocator.allocate(size) } {
            unsafe { ptr::write(object, GCObject::new(rtti, self.current_live_mark)); }
            unsafe{ (*ImmixSpace::get_block_ptr(object)).set_new_object(object); }
            ImmixSpace::set_gc_object(object);
            return Some(object);
        }
        return None;
    }

    pub fn prepare_collection(&mut self, evacuation: bool, cycle_collect: bool)
            -> CollectionType {
        self.collector.extend_all_blocks(self.allocator.get_all_blocks());
        self.collector.extend_all_blocks(self.overflow_allocator.get_all_blocks());
        self.collector.extend_all_blocks(self.evac_allocator.get_all_blocks());
        return self.collector.prepare_collection(evacuation, cycle_collect,
                                                 self.evac_allocator.evac_headroom());
    }

    pub fn collect(&mut self, collection_type: &CollectionType, roots: &[GCObjectRef]) {
        self.collector.collect(collection_type, roots, &mut self.evac_allocator,
                               !self.current_live_mark);
    }

    pub fn complete_collection(&mut self, collection_type: &CollectionType) {
        if collection_type.is_immix() {
            self.current_live_mark = !self.current_live_mark;
        }

        let (recyclable_blocks, evac_headroom) = self.collector.complete_collection();
        self.allocator.set_recyclable_blocks(recyclable_blocks);
        self.evac_allocator.extend_evac_headroom(evac_headroom);
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
