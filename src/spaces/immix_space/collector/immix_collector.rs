// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::RingBuf;

use spaces::immix_space::allocator::EvacAllocator;
use spaces::immix_space::ImmixSpace;
use gc_object::GCObjectRef;
use spaces::CollectionType;

pub struct ImmixCollector;

impl ImmixCollector {
    pub fn collect(collection_type: &CollectionType, roots: &[GCObjectRef],
                   evac_allocator: &mut EvacAllocator, next_live_mark: bool) {
        debug!("Start Immix collection with {} roots and next_live_mark: {}",
               roots.len(), next_live_mark);
        let mut object_queue: RingBuf<GCObjectRef> = roots.iter().map(|o| *o).collect();

        while let Some(object) =  object_queue.pop_front() {
            debug!("Process object {:p} in Immix closure", object);
            if !unsafe { (*object).set_marked(next_live_mark) } {
                ImmixSpace::set_gc_object(object);
                ImmixSpace::increment_lines(object);
                debug!("Object {:p} was unmarked: process children", object);
                let children = unsafe{ (*object).children() };
                for (num, mut child) in children.into_iter().enumerate() {
                    if let Some(new_child) = unsafe{ (*child).is_forwarded() } {
                        debug!("Child {:p} is forwarded to {:p}", child, new_child);
                        unsafe{ (*object).set_child(num, new_child); }
                    } else if !unsafe{ (*child).is_marked(next_live_mark) } {
                        if collection_type.is_evac() {
                            if let Some(new_child) = evac_allocator.maybe_evacuate(child) {
                                debug!("Evacuated child {:p} to {:p}", child, new_child);
                                unsafe{ (*object).set_child(num, new_child); }
                                child = new_child;
                            }
                        }
                        debug!("Push child {:p} into object queue", child);
                        object_queue.push_back(child);
                    }
                }
            }
        }
        debug!("Complete collection");
    }
}
