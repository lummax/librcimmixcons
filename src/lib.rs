// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![feature(libc)]
#![feature(os)]
#![feature(core)]
#![feature(std_misc)]
#![feature(collections)]
#![feature(link_llvm_intrinsics)]

extern crate libc;
use std::{mem, ptr};

pub use self::gc_object::{GCHeader, GCRTTI, GCObject, GCObjectRef};

mod macros;
mod constants;
mod gc_object;
mod spaces;
mod stack;

/// The `RCImmixCons` garbage collector.
///
/// This is the conservative reference counting garbage collector with the
/// immix heap partition schema.
///
/// The `allocate()` function will return a pointer to a `GCObject`. Please
/// see the documentation of `GCHeader`, `GCRTTI` and `GCObject` for details.
///
/// Always call `write_barrier()` on an object before modifying its members.
pub struct RCImmixCons {
    /// The different spaces of this garbage collector.
    spaces: spaces::Spaces,
}

impl RCImmixCons {
    /// Create a new `RCImmixCons`.
    pub fn new() -> RCImmixCons {
        return RCImmixCons {
            spaces: spaces::Spaces::new(),
        };
    }

    /// Allocate a new object described by the `rtti` or returns `None`.
    ///
    /// This may trigger a garbage collection if the allocation was not
    /// succussful. If there is still no memory to fullfill the allocation
    /// request return `None`.
    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        return self.spaces.allocate(rtti)
                   .or_else(|| { self.collect(true, true);
                                 self.spaces.allocate(rtti) });
    }

    /// Trigger a garbage collection.
    ///
    /// This will always run the referece counting collector. If `evacuation`
    /// is set the collectors will try to evacuate. If `cycle_collect` is set
    /// the immix tracing collector will be used.
    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        return self.spaces.collect(evacuation, cycle_collect);
    }

    /// A write barrier for the given `object`.
    ///
    /// Call this function before modifying the members of this object!
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
