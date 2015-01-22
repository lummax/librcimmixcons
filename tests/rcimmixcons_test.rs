// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate rcimmixcons;

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
    collector.collect(false, false);
}

