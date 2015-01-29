// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use constants::{BLOCK_SIZE, LINE_SIZE};
use gc_object::{GCRTTI, GCObjectRef};
use immix_collector::ImmixCollector;
use immix_space::ImmixSpace;
use rc_collector::RCCollector;
use stack;

pub struct Coordinator {
    immix_space: ImmixSpace,
    rc_collector: RCCollector,
}

impl Coordinator {
    pub fn new() -> Coordinator {
        return Coordinator {
            immix_space: ImmixSpace::new(),
            rc_collector: RCCollector::new(),
        };
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef>{
        // XXX use LOS if size > BLOCK_SIZE - LINE_SIZE
        assert!(unsafe{ (*rtti).object_size() } <= BLOCK_SIZE - LINE_SIZE);
        return self.immix_space.allocate(rtti)
                                  .or_else(|| { self.collect(true, true);
                                                self.allocate(rtti) });
    }

    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        debug!("Requested collection (evacuation={}, cycle_collect={})",
               evacuation, cycle_collect);
        let roots = stack::enumerate_roots(&mut self.immix_space);
        let perform_cc = self.immix_space.prepare_collection(evacuation, cycle_collect);
        self.rc_collector.collect(&mut self.immix_space, roots.as_slice());
        if perform_cc {
            ImmixCollector::collect(&mut self.immix_space, roots.as_slice());
        }
        self.immix_space.complete_collection();
        valgrind_assert_no_leaks!();
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        if self.immix_space.is_gc_object(object) {
            self.rc_collector.write_barrier(object);
        }
    }
}

