// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![feature(asm)]
#![feature(macro_rules)]

extern crate libc;
use std::{mem, ptr};

pub use self::gc_object::{GCHeader, GCObject, GCRTTI};

mod macros;
mod constants;
mod gc_object;
mod block_info;
mod block_allocator;
mod line_allocator;
mod rc_collector;
mod immix_collector;
mod stack;
mod posix;

pub struct RCImmixCons {
    line_allocator: line_allocator::LineAllocator,
    rc_collector: rc_collector::RCCollector,
}

impl RCImmixCons {
    pub fn new() -> RCImmixCons {
        let block_allocator = block_allocator::BlockAllocator::new();
        return RCImmixCons {
            line_allocator: line_allocator::LineAllocator::new(block_allocator),
            rc_collector: rc_collector::RCCollector::new(),
        };
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<*mut GCObject>{
        // XXX use LOS if size > BLOCK_SIZE - LINE_SIZE
        assert!(unsafe{ (*rtti).object_size() }
                <= constants::BLOCK_SIZE - constants::LINE_SIZE);
        return self.line_allocator.allocate(rtti);
    }

    pub fn collect(&mut self) {
        let roots = stack::enumerate_roots(&self.line_allocator);
        self.rc_collector.collect(&mut self.line_allocator, roots.as_slice());
        immix_collector::ImmixCollector::collect(&mut self.line_allocator,
                                                 roots.as_slice());
    }

    pub fn write_barrier(&mut self, object: *mut GCObject) {
        if self.line_allocator.is_gc_object(object) {
            self.rc_collector.write_barrier(object);
        }
    }
}

#[no_mangle]
pub extern fn rcx_create() -> *mut RCImmixCons {
    return unsafe { mem::transmute(box RCImmixCons::new()) };
}

#[no_mangle]
pub extern fn rcx_allocate(this: *mut RCImmixCons, rtti: *const GCRTTI)
    -> *mut GCObject {
    unsafe { return (*this).allocate(rtti).unwrap_or(ptr::null_mut()); }
}

#[no_mangle]
pub extern fn rcx_collect(this: *mut RCImmixCons) {
    unsafe { (*this).collect() };
}

#[no_mangle]
pub extern fn rcx_write_barrier(this: *mut RCImmixCons, object: *mut GCObject) {
    unsafe { (*this).write_barrier(object) };
}

#[no_mangle]
pub extern fn rcx_destroy(this: *mut RCImmixCons) {
    let _to_be_dropped: Box<RCImmixCons> = unsafe{ mem::transmute(this) };
}
