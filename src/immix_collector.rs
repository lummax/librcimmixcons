// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::RingBuf;

use gc_object::GCObjectRef;
use line_allocator::LineAllocator;

pub struct ImmixCollector;

impl ImmixCollector {
    pub fn collect(line_allocator: &mut LineAllocator, roots: &[GCObjectRef]) {
        let next_live_mark = !line_allocator.current_live_mark();
        debug!("Start Immix collection with {} roots and next_live_mark: {}",
               roots.len(), next_live_mark);
        line_allocator.prepare_immix_collection();
        let mut object_queue = RingBuf::new();
        for root in roots.iter().map(|o| *o) {
            unsafe{ (*root).set_pinned(true); }
            object_queue.push_back(root);
        }
        while let Some(object) =  object_queue.pop_front() {
            debug!("Process object {} in Immix closure", object);
            if !unsafe { (*object).set_marked(next_live_mark) } {
                line_allocator.set_gc_object(object);
                line_allocator.increment_lines(object);
                debug!("Object {} was unmarked: process children", object);
                let children = unsafe{ (*object).children() };
                for (num, mut child) in children.into_iter().enumerate() {
                    if let Some(new_child) = unsafe{ (*child).is_forwarded() } {
                        debug!("Child {} is forwarded to {}", child, new_child);
                        unsafe{ (*object).set_child(num, new_child); }
                    } else if !unsafe{ (*child).is_marked(next_live_mark) } {
                        if let Some(new_child) = line_allocator.maybe_evacuate(child) {
                            debug!("Evacuated child {} to {}", child, new_child);
                            unsafe{ (*object).set_child(num, new_child); }
                            child = new_child;
                        }
                        debug!("Push child {} into object queue", child);
                        object_queue.push_back(child);
                    }
                }
            }
        }
        for root in roots.iter() {
            unsafe{ (**root).set_pinned(false); }
        }
        line_allocator.complete_immix_collection();
        debug!("Complete collection");
    }
}
