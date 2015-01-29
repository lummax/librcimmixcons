// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod block_info;
mod block_allocator;
mod allocator;
mod collector;

pub use self::collector::ImmixCollector;
pub use self::collector::RCCollector;

use self::block_info::BlockInfo;

use std::{mem, ptr};

use constants::BLOCK_SIZE;
use gc_object::{GCRTTI, GCObject, GCObjectRef};
use stack;

pub struct ImmixSpace {
    allocator: allocator::Allocator,
    collector: collector::Collector,
    current_live_mark: bool,
    perform_evac: bool,
}

impl ImmixSpace {
    pub fn new() -> ImmixSpace {
        return ImmixSpace {
            allocator: allocator::Allocator::new(),
            collector: collector::Collector::new(),
            current_live_mark: false,
            perform_evac: false,
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
        return self.allocator.is_in_space(object);
    }

    pub fn current_live_mark(&self) -> bool {
        return self.current_live_mark;
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        if let Some(object) = self.allocator.allocate(size, self.perform_evac) {
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
        if let Some(new_object) = self.allocator.allocate(size, self.perform_evac) {
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

    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool,
                   rc_collector: &mut RCCollector) {

        let roots = stack::enumerate_roots(self);
        let evac_headroom = self.allocator.evac_headroom();
        let all_blocks = self.allocator.get_all_blocks();

        let (perform_cc, perform_evac)
            = self.collector.prepare_collection(evacuation, cycle_collect,
                all_blocks, self.allocator.block_allocator(), evac_headroom);
        self.perform_evac = perform_evac;

        rc_collector.collect(self, roots.as_slice());

        if perform_cc {
            ImmixCollector::collect(self, roots.as_slice());
            self.current_live_mark = !self.current_live_mark;
        }
        let (unavailable_blocks, recyclable_blocks, evac_headroom) =
            self.collector.complete_collection(self.allocator.block_allocator());

        self.perform_evac = false;
        self.allocator.set_unavailable_blocks(unavailable_blocks);
        self.allocator.set_recyclable_blocks(recyclable_blocks);
        self.allocator.extend_evac_headroom(evac_headroom);
        valgrind_assert_no_leaks!();
    }

    pub fn prepare_rc_collection(&mut self) {
        self.collector.prepare_rc_collection();
    }

    pub fn complete_rc_collection(&mut self) {
        self.collector.complete_rc_collection();
    }

    pub fn prepare_immix_collection(&mut self) {
        self.collector.prepare_immix_collection();
    }

    pub fn complete_immix_collection(&mut self) {
        self.collector.complete_immix_collection();
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
