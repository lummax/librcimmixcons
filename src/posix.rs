// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

pub mod setjmp {
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

pub mod pthread {
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
