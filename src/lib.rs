// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![feature(asm)]
#![feature(drain)]
#![feature(link_llvm_intrinsics)]

#![feature(plugin)]
#![plugin(clippy)]

//! This is an implementation of the `RCImmixCons` garbage collector.
//!
//! A conservative reference counting garbage collector with the immix heap
//! partition schema. For details please refer to:
//!
//! - S. M. S. Blackburn and K. K. S. McKinley. Immix: a mark-region garbage
//!   collector with space efficiency, fast collection, and mutator performance.
//!   ACM SIGPLAN Notices, 43(6):22, May 2008.
//! - R. Shahriyar, S. M. Blackburn, and D. Frampton. Down for the count?  Getting
//!   reference counting back in the ring. ACM SIGPLAN Notices, 47(11):73, Jan.
//!   2013.
//! - R. Shahriyar, S. M. Blackburn, and K. S. McKinley. Fast conservative garbage
//!   collection. In Proceedings of the 2014 ACM International Conference on
//!   Object Oriented Programming Systems Languages & Applications - OOPSLA ’14,
//!   pages 121-139, New York, New York, USA, Oct. 2014. ACM Press.
//! - R. Shahriyar, S. M. Blackburn, X. Yang, and K. S. McKinley. Taking off the
//!   gloves with reference counting Immix. ACM SIGPLAN Notices, 48(10):93-110,
//!   Nov. 2013.
//!
//! To use this garbage collector your objects must be structs derived from
//! `GCObject`. Allocation and collection is done using `RCImmixCons`.

extern crate libc;
extern crate bit_set;
extern crate vec_map;

use std::ptr;

pub use self::gc_object::{GCHeader, GCRTTI, GCObject, GCObjectRef};

mod macros;
mod constants;
mod gc_object;
mod spaces;
mod stack;

/// The `RCImmixCons` garbage collector.
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
        RCImmixCons {
            spaces: spaces::Spaces::new(),
        }
    }

    /// Allocate a new object described by the `rtti` or returns `None`.
    ///
    /// This may trigger a garbage collection if the allocation was not
    /// succussful. If there is still no memory to fullfill the allocation
    /// request return `None`.
    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        self.spaces.allocate(rtti)
            .or_else(|| { self.collect(true, true);
                          self.spaces.allocate(rtti) })
    }

    /// Trigger a garbage collection.
    ///
    /// This will always run the referece counting collector. If `evacuation`
    /// is set the collectors will try to evacuate. If `cycle_collect` is set
    /// the immix tracing collector will be used.
    #[allow(unused_variables)]
    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        // Calling this function befor **ANYTHING** is important to save the
        // callee save registers on the stack.
        let registers = stack::Stack::get_registers();
        self.spaces.collect(evacuation, cycle_collect)
    }

    /// Set an address to an object reference as static root.
    ///
    /// Use this to mark global/static variables as roots. This is needed, if
    /// the pointer to a garbage collected object does not reside on the stack
    /// or in any register.
    pub fn set_static_root(&mut self, address: *const GCObjectRef) {
        self.spaces.set_static_root(address);
    }

    /// A write barrier for the given `object`.
    ///
    /// Call this function before modifying the members of this object!
    pub fn write_barrier(&mut self, object: GCObjectRef) {
        if self.spaces.write_barrier(object) {
            self.collect(false, false);
        }
    }
}

#[no_mangle]
#[doc(hidden)]
pub extern fn rcx_create() -> *mut RCImmixCons {
    Box::into_raw(Box::new(RCImmixCons::new()))
}

#[no_mangle]
#[doc(hidden)]
pub extern fn rcx_allocate(this: *mut RCImmixCons, rtti: *const GCRTTI)
    -> GCObjectRef {
    unsafe { (*this).allocate(rtti).unwrap_or(ptr::null_mut()) }
}

#[no_mangle]
#[doc(hidden)]
pub extern fn rcx_collect(this: *mut RCImmixCons, evacuation: bool, cycle_collect: bool) {
    unsafe { (*this).collect(evacuation, cycle_collect) };
}

#[no_mangle]
#[doc(hidden)]
pub extern fn rcx_set_static_root(this: *mut RCImmixCons, address: *const GCObjectRef) {
    unsafe { (*this).set_static_root(address) };
}

#[no_mangle]
#[doc(hidden)]
pub extern fn rcx_write_barrier(this: *mut RCImmixCons, object: GCObjectRef) {
    unsafe { (*this).write_barrier(object) };
}

#[no_mangle]
#[doc(hidden)]
pub extern fn rcx_destroy(this: *mut RCImmixCons) {
    let _to_be_dropped = unsafe{ Box::from_raw(this) };
}
