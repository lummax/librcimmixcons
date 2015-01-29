// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![allow(unstable)]
#![feature(link_llvm_intrinsics)]

extern crate libc;
use std::{mem, ptr};

pub use self::gc_object::{GCHeader, GCRTTI, GCObject, GCObjectRef};

mod macros;
mod constants;
mod gc_object;
mod coordinator;
mod immix_space;
mod rc_collector;
mod immix_collector;
mod stack;

pub struct RCImmixCons {
    coordinator: coordinator::Coordinator,
}

impl RCImmixCons {
    pub fn new() -> RCImmixCons {
        return RCImmixCons {
            coordinator: coordinator::Coordinator::new(),
        };
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        return self.coordinator.allocate(rtti);
    }

    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        return self.coordinator.collect(evacuation, cycle_collect);
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        return self.coordinator.write_barrier(object);
    }
}

#[no_mangle]
pub extern fn rcx_create() -> *mut RCImmixCons {
    return unsafe { mem::transmute(Box::new(RCImmixCons::new())) };
}

#[no_mangle]
pub extern fn rcx_allocate(this: *mut RCImmixCons, rtti: *const GCRTTI)
    -> GCObjectRef {
    unsafe { return (*this).allocate(rtti).unwrap_or(ptr::null_mut()); }
}

#[no_mangle]
pub extern fn rcx_collect(this: *mut RCImmixCons, evacuation: bool, cycle_collect: bool) {
    unsafe { (*this).collect(evacuation, cycle_collect) };
}

#[no_mangle]
pub extern fn rcx_write_barrier(this: *mut RCImmixCons, object: GCObjectRef) {
    unsafe { (*this).write_barrier(object) };
}

#[no_mangle]
pub extern fn rcx_destroy(this: *mut RCImmixCons) {
    let _to_be_dropped: Box<RCImmixCons> = unsafe{ mem::transmute(this) };
}
