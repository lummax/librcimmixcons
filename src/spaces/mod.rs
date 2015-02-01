// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod immix_space;
mod collector;

use self::immix_space::ImmixSpace;
use self::collector::Collector;

use constants::LARGE_OBJECT;
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
    immix_space: ImmixSpace,
    collector: Collector,
    current_live_mark: bool,
}

impl Spaces {
    pub fn new() -> Spaces {
        return Spaces {
            immix_space: ImmixSpace::new(),
            collector: Collector::new(),
            current_live_mark: false,
        };
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.immix_space.is_gc_object(object);
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        if self.is_gc_object(object) {
            self.collector.write_barrier(object);
        }
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef>{
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        return self.immix_space.allocate(rtti)
               .or_else(|| { self.collect(true, true); self.allocate(rtti) });
    }

    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        debug!("Requested collection (evacuation={}, cycle_collect={})",
               evacuation, cycle_collect);

        let roots = stack::enumerate_roots(self);
        self.collector.extend_all_blocks(self.immix_space.get_all_blocks());
        let collection_type = self.collector.prepare_collection(evacuation,
                                cycle_collect,
                                self.immix_space.available_blocks(),
                                self.immix_space.total_blocks(),
                                self.immix_space.evac_headroom());

        if collection_type.is_immix() {
            for root in roots.iter().map(|o| *o) {
                unsafe{ (*root).set_pinned(true); }
            }
        }

        self.collector.collect(&collection_type, roots.as_slice(),
                               self.immix_space.evac_allocator(),
                               !self.current_live_mark);

        if collection_type.is_immix() {
            self.current_live_mark = !self.current_live_mark;
            self.immix_space.set_current_live_mark(self.current_live_mark);

            for root in roots.iter() {
                unsafe{ (**root).set_pinned(false); }
            }
        }

        self.collector.complete_collection(&collection_type, &mut self.immix_space);
        valgrind_assert_no_leaks!();
    }
}

