use std::collections::RingBuf;

use gc_object::GCObject;
use line_allocator::LineAllocator;

pub struct ImmixCollector;

impl ImmixCollector {
    pub fn collect(line_allocator: &mut LineAllocator, roots: &[*mut GCObject]) {
        debug!("Start Immix collection with {} roots", roots.len());
        line_allocator.clear_line_counts();
        line_allocator.clear_object_map();
        let current_live_mark = line_allocator.current_live_mark();
        let mut object_queue = roots.iter().map(|o| *o)
                                    .collect::<RingBuf<*mut GCObject>>();
        loop {
            match object_queue.pop_front() {
                None => break,
                Some(object) => {
                    if !unsafe { (*object).set_marked(current_live_mark) } {
                        debug!("Process object {} in Immix closure", object);
                        line_allocator.set_gc_object(object);
                        line_allocator.increment_lines(object);
                        for child in unsafe{ (*object).children() }.into_iter() {
                            if !unsafe{ (*child).is_marked(current_live_mark) } {
                                object_queue.push_back(child);
                            }
                        }
                    }
                }
            }
        }
        line_allocator.invert_live_mark();
        debug!("Sweep and return empty blocks (Immix)");
        line_allocator.return_empty_blocks();
    }
}
