extern crate libc;

use posix::pthread;
use posix::setjmp;

use std::collections::HashSet;
use std::{ptr, mem};

use gc_object::GCObject;
use line_allocator::LineAllocator;

#[inline(always)]
#[cfg(target_arch = "x86")]
#[allow(unused_assignments)]
fn get_stack_top() -> *mut u8 {
    let mut top = ptr::null_mut();
    unsafe { asm!("movl %esp, %eax" : "=eax" (top)); }
    return top;
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
#[allow(unused_assignments)]
fn get_stack_top() -> *mut u8 {
    let mut top = ptr::null_mut();
    unsafe { asm!("movq %rsp, %rax" : "=rax" (top)); }
    return top;
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
        return Some(stackaddr.offset(stacksize as int) as *mut u8);
    }
}

#[inline(always)]
fn get_stack() -> Option<(*mut u8, *mut u8)> {
    return get_stack_bottom().map(|bottom| (get_stack_top(), bottom))
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
pub fn enumerate_roots(line_allocator: &LineAllocator) -> Vec<*mut GCObject> {
    let jmp_buf = save_registers();
    if let Some((top, bottom)) = get_stack() {
        return range(top as uint, unsafe{ bottom.offset(-7) } as uint )
            .map(|e| unsafe{ *(e as *const *mut GCObject) })
            .filter(|e| line_allocator.is_gc_object(*e))
            .collect::<HashSet<*mut GCObject>>()
            .into_iter().collect();
    }
    return Vec::new();
}
