// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod immix_space;

use constants::{BLOCK_SIZE, LINE_SIZE};
use gc_object::{GCRTTI, GCObjectRef};
use stack;

pub enum CollectionType {
    RCCollection,
    RCEvacCollection,
    ImmixCollection,
    ImmixEvacCollection,
}

impl CollectionType {
    pub fn is_evac(&self) -> bool {
        use self::CollectionType::{RCEvacCollection, ImmixEvacCollection};
        return match *self {
            RCEvacCollection | ImmixEvacCollection => true,
            _ => false,
        }
    }

    pub fn is_immix(&self) -> bool {
        use self::CollectionType::{ImmixCollection, ImmixEvacCollection};
        return match *self {
            ImmixCollection | ImmixEvacCollection => true,
            _ => false,
        }
    }
}

pub struct Spaces {
    immix_space: immix_space::ImmixSpace,
}

impl Spaces {
    pub fn new() -> Spaces {
        return Spaces {
            immix_space: immix_space::ImmixSpace::new(),
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
        let roots = stack::enumerate_roots(self);
        let collection_type = self.immix_space.prepare_collection(evacuation,
                                                                  cycle_collect);
        self.immix_space.collect(&collection_type, roots.as_slice());
        self.immix_space.complete_collection(&collection_type);
        valgrind_assert_no_leaks!();
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.immix_space.is_gc_object(object);
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        self.immix_space.write_barrier(object);
    }
}

