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
mod spaces;
mod stack;

pub struct RCImmixCons {
    spaces: spaces::Spaces,
}

impl RCImmixCons {
    pub fn new() -> RCImmixCons {
        return RCImmixCons {
            spaces: spaces::Spaces::new(),
        };
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        return self.spaces.allocate(rtti);
    }

    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        return self.spaces.collect(evacuation, cycle_collect);
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        return self.spaces.write_barrier(object);
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
