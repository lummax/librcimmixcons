// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::collections::HashSet;
use std::{ptr, mem};

use gc_object::GCObjectRef;
use spaces::ImmixSpace;

mod setjmp {
    extern crate libc;

    pub use self::arch::jmp_buf;

    extern {
        fn _setjmp(env: *mut jmp_buf) -> libc::c_int;
    }

    pub unsafe fn setjmp(env: *mut jmp_buf) -> libc::c_int {
        return _setjmp(env);
    }

    #[cfg(target_arch = "x86_64")]
    mod arch {
        #[repr(C)]
        pub struct jmp_buf {
            _data: [u64; 25]
        }
    }

    #[cfg(target_arch = "x86")]
    mod arch {
        pub type jmp_buf = [[u32; 39]; 1];
    }
}

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

#[inline(always)]
fn get_stack_top() -> *mut u8 {
    extern {
        #[link_name = "llvm.frameaddress"]
        fn frameaddress(level: i32) -> *mut u8;
    }
    unsafe{ return frameaddress(0); }
}

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

#[inline(always)]
#[cfg(all(target_os = "linux", any(target_arch = "x86", target_arch = "x86_64")))]
fn save_registers() -> setjmp::jmp_buf {
    unsafe {
        let mut jmp_buf: setjmp::jmp_buf = mem::zeroed();
        setjmp::setjmp(&mut jmp_buf);
        return jmp_buf;
    }
}

#[allow(unused_variables)]
pub fn enumerate_roots(immix_space: &mut ImmixSpace) -> Vec<GCObjectRef> {
    let jmp_buf = save_registers();
    if let Some(bottom) = get_stack_bottom() {
        let top = get_stack_top();
        let stack_size = (bottom as usize) - (top as usize) - 8;
        debug!("Scanning stack of size {} ({:p} - {:p})", stack_size, top, bottom);
        return (0..stack_size)
            .map(|o| unsafe{ *(top.offset(o as isize) as *const GCObjectRef) })
            .filter(|o| !o.is_null() && immix_space.is_gc_object(*o))
            .collect::<HashSet<GCObjectRef>>()
            .into_iter().collect();
    }
    return Vec::new();
}
