// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::RingBuf;

use gc_object::GCObjectRef;
use spaces::ImmixSpace;

pub struct RCCollector {
    old_root_buffer: RingBuf<GCObjectRef>,
    decrement_buffer: RingBuf<GCObjectRef>,
    modified_buffer: RingBuf<GCObjectRef>,
    perform_evac: bool,
}

impl RCCollector {
    pub fn new() -> RCCollector {
        return RCCollector {
            old_root_buffer: RingBuf::new(),
            decrement_buffer: RingBuf::new(),
            modified_buffer: RingBuf::new(),
            perform_evac: false,
        };
    }

    pub fn collect(&mut self, immix_space: &mut ImmixSpace, perform_evac: bool,
                   roots: &[GCObjectRef]) {
        debug!("Start RC collection");
        self.perform_evac = perform_evac;
        self.process_old_roots();
        self.process_current_roots(immix_space, roots);
        self.process_mod_buffer(immix_space);
        self.process_decrement_buffer();
        debug!("Complete collection");
    }

    pub fn write_barrier(&mut self, object: GCObjectRef) {
        debug!("Write barrier on object {:p}", object);
        self.modified(object);
        for child in unsafe{ (*object).children() }.into_iter() {
            self.decrement(child);
        }
        unsafe{ (*object).set_logged(true); }
    }
}

impl RCCollector {
    fn modified(&mut self, object: GCObjectRef) {
        debug!("Push object {:p} into mod buffer", object);
        self.modified_buffer.push_back(object);
    }

    fn decrement(&mut self, object: GCObjectRef) {
        debug!("Push object {:p} into dec buffer", object);
        self.decrement_buffer.push_back(object);
    }

    fn increment(&mut self, immix_space: &mut ImmixSpace,
                 object: GCObjectRef, try_evacuate: bool) -> Option<GCObjectRef> {
        debug!("Increment object {:p}", object);
        if unsafe{ (*object).increment() } {
            if try_evacuate && self.perform_evac {
                if let Some(new_object) = immix_space.maybe_evacuate(object) {
                    debug!("Evacuated object {:p} to {:p}", object, new_object);
                    ImmixSpace::decrement_lines(object);
                    ImmixSpace::increment_lines(new_object);
                    self.modified(new_object);
                    return Some(new_object);
                }
            }
            ImmixSpace::increment_lines(object);
            self.modified(object);
        }
        return None;
    }

    fn process_old_roots(&mut self) {
        debug!("Process old roots (size {})", self.old_root_buffer.len());
        self.decrement_buffer.extend(self.old_root_buffer.drain());
    }

    fn process_current_roots(&mut self, immix_space: &mut ImmixSpace,
                             roots: &[GCObjectRef]) {
        debug!("Process current roots (size {})", roots.len());
        for root in roots.iter().map(|o| *o) {
            debug!("Process root object: {:p}", root);
            self.increment(immix_space, root, false);
            self.old_root_buffer.push_back(root);
        }
    }

    fn process_mod_buffer(&mut self, immix_space: &mut ImmixSpace) {
        debug!("Process mod buffer (size {})", self.modified_buffer.len());
        while let Some(object) = self.modified_buffer.pop_front() {
            debug!("Process object {:p} in mod buffer", object);
            unsafe { (*object).set_logged(false); }
            ImmixSpace::set_gc_object(object);
            let children = unsafe{ (*object).children() };
            for (num, child) in children.into_iter().enumerate() {
                if let Some(new_child) = unsafe{ (*child).is_forwarded() } {
                    debug!("Child {:p} is forwarded to {:p}", child, new_child);
                    unsafe{ (*object).set_child(num, new_child); }
                    self.increment(immix_space, child, false);
                } else {
                    if let Some(new_child) = self.increment(immix_space,
                                                            child, true) {
                        unsafe{ (*object).set_child(num, new_child); }
                    }
                }
            }
        }
    }

    fn process_decrement_buffer(&mut self) {
        debug!("Process dec buffer (size {})", self.decrement_buffer.len());
        while let Some(object) =  self.decrement_buffer.pop_front() {
            debug!("Process object {:p} in dec buffer", object);
            if unsafe{ (*object).decrement() } {
                ImmixSpace::unset_gc_object(object);
                ImmixSpace::decrement_lines(object);
                for child in unsafe{ (*object).children() }.into_iter() {
                    self.decrement(child);
                }
                valgrind_freelike!(object);
            }
        }
    }
}
