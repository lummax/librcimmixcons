// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod immix_space;
mod large_object_space;
mod collector;

use self::immix_space::ImmixSpace;
use self::large_object_space::LargeObjectSpace;
use self::collector::Collector;

use constants::LARGE_OBJECT;
use gc_object::{GCRTTI, GCObjectRef};
use stack::Stack;

/// The type of collection that will be performed.
pub enum CollectionType {
    /// A simple reference counting collection.
    RCCollection,

    /// A reference counting collection with proactive opportunistic
    /// evacuation.
    RCEvacCollection,

    /// A reference counting collection followed by the immix tracing (cycle)
    /// collection.
    ImmixCollection,

    /// A reference counting collection followed by the immix tracing (cycle)
    /// collection. Both with opportunistict evacuation.
    ImmixEvacCollection,
}

impl CollectionType {
    /// Returns if this `CollectionType` is an evacuating collection.
    pub fn is_evac(&self) -> bool {
        use self::CollectionType::{RCEvacCollection, ImmixEvacCollection};
        return match *self {
            RCEvacCollection | ImmixEvacCollection => true,
            _ => false,
        }
    }

    /// Returns if this `CollectionType` is a cycle collecting collection.
    pub fn is_immix(&self) -> bool {
        use self::CollectionType::{ImmixCollection, ImmixEvacCollection};
        return match *self {
            ImmixCollection | ImmixEvacCollection => true,
            _ => false,
        }
    }
}

/// The `Spaces` contains the different garbage collector spaces in which
/// objects can be allocated that are managed by some collector.
pub struct Spaces {
    /// The stack of the one execution thread.
    stack: Stack,

    /// The default immix space.
    immix_space: ImmixSpace,

    /// The large object space for objects greater than `LARGE_OBJECT`.
    large_object_space: LargeObjectSpace,

    /// The collectors.
    collector: Collector,

    /// The current live mark.
    ///
    /// During allocation of objects this value is used as the `mark` state of
    /// new objects. During allocation the value is negated and used to mark
    /// objects during the tracing mark phase. This way the newly allocated
    /// objects are always initialized with the last `mark` state with will be
    /// flipped if they are reached is the mark phase.
    current_live_mark: bool,
}

impl Spaces {
    /// Create a new `Spaces`.
    pub fn new() -> Spaces {
        return Spaces {
            stack: Stack::new(),
            immix_space: ImmixSpace::new(),
            large_object_space: LargeObjectSpace::new(),
            collector: Collector::new(),
            current_live_mark: false,
        };
    }


    /// Return if the given address is valid in any of the managed spaces.
    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.immix_space.is_gc_object(object)
               || self.large_object_space.is_gc_object(object);
    }

    /// Set an address of an object reference as static root.
    pub fn set_static_root(&mut self, address: *const GCObjectRef) {
        self.stack.set_static_root(address);
    }

    /// A write barrier for the given `object` used with the `RCCollector`.
    pub fn write_barrier(&mut self, object: GCObjectRef) {
        if self.is_gc_object(object) {
            self.collector.write_barrier(object);
        }
    }

    /// Allocate a new object described by the `rtti` or returns `None` if
    /// there is no memory left to fullfill the allocation request.
    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef>{
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        return if size < LARGE_OBJECT { self.immix_space.allocate(rtti) }
               else { self.large_object_space.allocate(rtti) };
    }

    /// Collect the roots using `Stack::enumerate_roots()` and filter them for
    /// validity in `LargeObjectSpace` or `ImmixSpace`.
    fn collect_roots(&self) -> Vec<GCObjectRef> {
        let los_filter = self.large_object_space.is_gc_object_filter();
        let immix_filter = self.immix_space.is_gc_object_filter();
        return self.stack.enumerate_roots().into_iter()
                         .filter(|o| los_filter(*o) || immix_filter(*o))
                         .collect();
    }

    /// Trigger a garbage collection.
    ///
    /// This will always run the referece counting collector. If `evacuation`
    /// is set the collectors will try to evacuate. If `cycle_collect` is set
    /// the immix tracing collector will be used.
    pub fn collect(&mut self, evacuation: bool, cycle_collect: bool) {
        debug!("Requested collection (evacuation={}, cycle_collect={})",
               evacuation, cycle_collect);

        let roots = self.collect_roots();
        self.collector.extend_all_blocks(self.immix_space.get_all_blocks());

        for root in roots.iter().map(|o| *o) {
            unsafe{ (*root).set_pinned(true); }
        }

        let collection_type = self.collector.prepare_collection(evacuation,
                                cycle_collect,
                                self.immix_space.available_blocks(),
                                self.immix_space.total_blocks(),
                                self.immix_space.evac_headroom());
        self.collector.collect(&collection_type, roots.as_slice(),
                               &mut self.immix_space,
                               &mut self.large_object_space,
                               !self.current_live_mark);
        self.collector.complete_collection(&collection_type, &mut self.immix_space,
                                           &mut self.large_object_space);

        for root in roots.iter() {
            unsafe{ (**root).set_pinned(false); }
        }
        if collection_type.is_immix() {
            self.current_live_mark = !self.current_live_mark;
            self.immix_space.set_current_live_mark(self.current_live_mark);
            self.large_object_space.set_current_live_mark(self.current_live_mark);

        }
        valgrind_assert_no_leaks!();
    }
}

