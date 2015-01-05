// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![macro_escape]

#[macro_export]
macro_rules! debug(
    ($($args:tt)*) => (
        if cfg!(not(ndebug)) {
            println!("({}:{}) {}", Path::new(file!()).filename_str().unwrap(),
                     line!(), format_args!($($args)*));
        }
    )
);

#[cfg(feature = "valgrind")]
pub mod valgrind {
    extern crate vgrs;
    pub use self::vgrs::memcheck::{malloclike_block, freelike_block, do_quick_leak_check, count_leaks};
}

#[cfg(not(feature = "valgrind"))]
#[allow(unused_variables)]
pub mod valgrind {
    pub struct LeakCount {
        pub leaked: uint,
    }
    pub unsafe fn malloclike_block(addr: *const (), size: uint, redzone: uint, is_zeroed: bool) { }
    pub unsafe fn freelike_block(addr: *const (), redzone: uint) { }
    pub unsafe fn do_quick_leak_check() { }
    pub unsafe fn count_leaks() -> LeakCount { return  LeakCount { leaked: 0 } }
}

#[macro_export]
macro_rules! valgrind_malloclike(
    ($addr:expr, $size:expr) => (
        if cfg!(feature = "valgrind") {
            unsafe{
                ::macros::valgrind::malloclike_block($addr as *const (), $size, 0, true);
            }
        }
    )
);

#[macro_export]
macro_rules! valgrind_freelike(
    ($addr:expr) => (
        if cfg!(feature = "valgrind") {
            unsafe{
                ::macros:: valgrind::freelike_block($addr as *const (), 0);
            }
        }
    )
);

#[macro_export]
macro_rules! valgrind_assert_no_leaks(
    () => (
        if cfg!(feature = "valgrind") {
            unsafe{
                if cfg!(not(ndebug)) {
                    ::macros::valgrind::do_quick_leak_check();
                }
                let leak_count = ::macros::valgrind::count_leaks();
                assert!(leak_count.leaked == 0, "Valgrind reported leaked memory");
            }
        }
    )
);
