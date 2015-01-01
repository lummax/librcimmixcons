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
