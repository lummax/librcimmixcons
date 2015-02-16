// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::collections::HashSet;
use std::{ptr, mem};

use gc_object::GCObjectRef;
use spaces::Spaces;

/// Abstractions over the stack to scan the stack and the registers for
/// garbage collection roots.

mod pthread {
    extern crate libc;

    extern {
        pub fn pthread_self() -> libc::pthread_t;
        pub fn pthread_getattr_np(native: libc::pthread_t,
                                  attr: *mut libc::pthread_attr_t) -> libc::c_int;
        pub fn pthread_attr_getstack(attr: *const libc::pthread_attr_t,
                                     stackaddr: *mut *mut libc::c_void,
                                     stacksize: *mut libc::size_t) -> libc::c_int;
        pub fn pthread_attr_destroy(attr: *mut libc::pthread_attr_t) -> libc::c_int;
    }
}

/// Return the top of the stack.
///
/// See the `llvm.frameaddress` intrinsic for details.
#[inline(always)]
fn get_stack_top() -> *mut u8 {
    extern {
        #[link_name = "llvm.frameaddress"]
        fn frameaddress(level: i32) -> *mut u8;
    }
    unsafe{ return frameaddress(0); }
}

/// Return the bottom of the stack.
///
/// This will be the lowest addressable address of the current threads stack
/// buffer minus the buffer size.
#[inline(always)]
#[cfg(target_os = "linux")]
fn get_stack_bottom() -> Option<*mut u8> {
    unsafe {
        let mut attr: libc::pthread_attr_t = mem::zeroed();
        if pthread::pthread_getattr_np(pthread::pthread_self(), &mut attr) != 0 {
            return None;
        }
        let mut stackaddr = ptr::null_mut();
        let mut stacksize = 0;
        if pthread::pthread_attr_getstack(&attr, &mut stackaddr, &mut stacksize) != 0 {
            return None;
        }
        pthread::pthread_attr_destroy(&mut attr);
        return Some(stackaddr.offset(stacksize as isize) as *mut u8);
    }
}

/// Get the contents of the registers
#[allow(unused_assignments)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn get_registers() -> Vec<GCObjectRef> {
    let mut rbx = ptr::null_mut(); unsafe{ asm!("movq %rbx, %rax": "=rax" (rbx));}
    let mut rsp = ptr::null_mut(); unsafe{ asm!("movq %rsp, %rax": "=rax" (rsp));}
    let mut rbp = ptr::null_mut(); unsafe{ asm!("movq %rbp, %rax": "=rax" (rbp));}
    let mut r12 = ptr::null_mut(); unsafe{ asm!("movq %r12, %rax": "=rax" (r12));}
    let mut r13 = ptr::null_mut(); unsafe{ asm!("movq %r13, %rax": "=rax" (r13));}
    let mut r14 = ptr::null_mut(); unsafe{ asm!("movq %r14, %rax": "=rax" (r14));}
    let mut r15 = ptr::null_mut(); unsafe{ asm!("movq %r15, %rax": "=rax" (r15));}
    let registers = vec![rbx, rsp, rbp, r12, r13, r14, r15];
    debug!("Register values: {:?}", registers);
    return registers;
}

/// Scan the stack and registers for garbage collection roots.
///
/// This will retrieve the callee save registers and validate all non-null
/// values on the stack as possible garbage collection roots using the
/// supplied `Spaces` (`Spaces.is_gc_object()`)
pub fn enumerate_roots(spaces: &mut Spaces) -> Vec<GCObjectRef> {
    if let Some(bottom) = get_stack_bottom() {
        let top = get_stack_top();
        let stack_size = (bottom as usize) - (top as usize) - 8;
        debug!("Scanning stack of size {} ({:p} - {:p})", stack_size, top, bottom);
        return (0..stack_size)
            .map(|o| unsafe{ *(top.offset(o as isize) as *const GCObjectRef) })
            .chain(get_registers().into_iter())
            .chain(spaces.static_roots().iter().map(|o| unsafe{ **o }))
            .filter(|o| !o.is_null() && spaces.is_gc_object(*o))
            .collect::<HashSet<GCObjectRef>>()
            .into_iter().collect();
    }
    return Vec::new();
}
