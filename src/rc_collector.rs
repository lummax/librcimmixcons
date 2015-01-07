// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::RingBuf;

use gc_object::GCObjectRef;
use line_allocator::LineAllocator;

pub struct RCCollector {
    old_root_buffer: RingBuf<GCObjectRef>,
    decrement_buffer: RingBuf<GCObjectRef>,
    modified_buffer: RingBuf<GCObjectRef>,
}

impl RCCollector {
    pub fn new() -> RCCollector {
        return RCCollector {
            old_root_buffer: RingBuf::new(),
            decrement_buffer: RingBuf::new(),
            modified_buffer: RingBuf::new(),
        };
    }

    pub fn collect(&mut self, line_allocator: &mut LineAllocator,
                   roots: &[GCObjectRef]) {
        debug!("Start RC collection");
        self.process_old_roots();
        self.process_current_roots(line_allocator, roots);
        self.process_mod_buffer(line_allocator);
        self.process_decrement_buffer(line_allocator);
        debug!("Complete collection");
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        debug!("Write barrier on object {}", object);
        self.modified(object);
        for child in unsafe{ (*object).children() }.into_iter() {
            self.decrement(child);
        }
        unsafe{ (*object).set_logged(true); }
    }
}

impl RCCollector {
    fn modified(&mut self, object: GCObjectRef) {
        debug!("Push object {} into mod buffer", object);
        self.modified_buffer.push_back(object);
    }

    fn decrement(&mut self, object: GCObjectRef) {
        debug!("Push object {} into dec buffer", object);
        self.decrement_buffer.push_back(object);
    }

    fn increment(&mut self, line_allocator: &mut LineAllocator, object: GCObjectRef) {
        debug!("Increment object {}", object);
        if unsafe{ (*object).increment() } {
            line_allocator.increment_lines(object);
            self.modified(object);
        }
    }

    fn process_old_roots(&mut self) {
        debug!("Process old roots (size {})", self.old_root_buffer.len());
        self.decrement_buffer.extend(self.old_root_buffer.drain());
    }

    fn process_current_roots(&mut self, line_allocator: &mut LineAllocator,
                             roots: &[GCObjectRef]) {
        debug!("Process current roots (size {})", roots.len());
        for root in roots.iter().map(|o| *o) {
            debug!("Process root object: {}", root);
            self.increment(line_allocator, root);
            self.old_root_buffer.push_back(root);
        }
    }

    fn process_mod_buffer(&mut self, line_allocator: &mut LineAllocator) {
        debug!("Process mod buffer (size {})", self.modified_buffer.len());
        while let Some(object) = self.modified_buffer.pop_front() {
            debug!("Process object {} in mod buffer", object);
            unsafe { (*object).set_logged(false); }
            for child in unsafe{ (*object).children() }.into_iter() {
                self.increment(line_allocator, child);
            }
        }
    }

    fn process_decrement_buffer(&mut self, line_allocator: &mut LineAllocator) {
        debug!("Process dec buffer (size {})", self.decrement_buffer.len());
        while let Some(object) =  self.decrement_buffer.pop_front() {
            debug!("Process object {} in dec buffer", object);
            if unsafe{ (*object).decrement() } {
                line_allocator.unset_gc_object(object);
                line_allocator.decrement_lines(object);
                for child in unsafe{ (*object).children() }.into_iter() {
                    self.decrement(child);
                }
                valgrind_freelike!(object);
            }
        }
    }
}
