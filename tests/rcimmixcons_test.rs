// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate rcimmixcons;
extern crate glob;
use std::io::Command;

macro_rules! run_c_file (
    ($name:ident) => (
        #[test]
        fn $name() {
            let library = glob::glob("target/librcimmixcons-*.so")
                .next().and_then(|p| p.filestem_str().map(|s| s.to_string()))
                .map(|s| s.replace("lib", "")).unwrap();
            let output = format!("target/{}", stringify!($name));
            let compile_success = Command::new("clang")
                .arg(format!("tests/{}.c", stringify!($name)))
                .arg("-L").arg("target").arg("-l").arg(library)
                .arg("-o").arg(&output).status().unwrap().success();
            assert!(compile_success);
            let run_success = Command::new(output)
                .env("LD_LIBRARY_PATH", "target").status().unwrap().success();
            assert!(run_success);
        }
    );
);

run_c_file!(simple_ffi_test);
run_c_file!(simple_closure_test);
run_c_file!(simple_overflow_test);
run_c_file!(simple_rc_collection_test);
run_c_file!(simple_rc_collection_and_recycle_test);
run_c_file!(simple_immix_collection_and_recycle_test);
// XXX disabled because of a weird rust bug where the tests hang
//run_c_file!(simple_rc_evacuation_test);
//run_c_file!(simple_immix_evacuation_test);

#[test]
#[allow(unused_variables)]
fn simple_allocate_test() {
    let mut collector = rcimmixcons::RCImmixCons::new();
    let rtti = rcimmixcons::GCRTTI::new(128, 0);
    let chunck1 = collector.allocate(&rtti).unwrap();
    let chunck2 = collector.allocate(&rtti).unwrap();
    let chunck3 = collector.allocate(&rtti).unwrap();
    let chunck4 = collector.allocate(&rtti).unwrap();
    let chunck5 = collector.allocate(&rtti).unwrap();
    collector.collect();
}

