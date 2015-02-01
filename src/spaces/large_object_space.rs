// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::collections::HashSet;
use std::ptr;

use gc_object::{GCRTTI, GCObject, GCObjectRef};

pub struct LargeObjectSpace  {
    objects: HashSet<GCObjectRef>,
    current_live_mark: bool,
}

impl LargeObjectSpace  {
    pub fn new() -> LargeObjectSpace {
        return LargeObjectSpace {
            objects: HashSet::new(),
            current_live_mark: false,
        };
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.objects.contains(&object);
    }

    pub fn unset_gc_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_gc_object(object),
                      "unset_gc_object() on invalid object {:p}", object);
        debug!("Unset object {:p} as los object", object);
        self.objects.remove(&object);
    }

    pub fn set_current_live_mark(&mut self, current_live_mark: bool) {
        self.current_live_mark = current_live_mark;
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        let object = unsafe{ libc::malloc(size as u64) } as GCObjectRef;
        if !object.is_null() {
            unsafe { ptr::write(object, GCObject::new(rtti, self.current_live_mark)); }
            self.objects.insert(object);
            return Some(object);
        }
        return None;
    }

    pub fn free(&self, object: GCObjectRef) {
        debug_assert!(self.is_gc_object(object),
                      "free() on invalid object {:p}", object);
        debug!("Free object {:p}", object);
        unsafe{ libc::free(object as *mut libc::c_void); }
    }

    pub fn sweep(&mut self) {
        let next_live_mark = !self.current_live_mark;
        debug!("Sweep LOS with next_live_mark={}", next_live_mark);
        for object in self.objects.iter().map(|&o| o)
                          .filter(|&o| unsafe{ !(*o).is_marked(next_live_mark) }) {
            self.free(object);
        }
        self.objects = self.objects.drain()
                           .filter(|&o| unsafe{ (*o).is_marked(next_live_mark) })
                           .collect();
    }
}

impl Drop for LargeObjectSpace {
    fn drop(&mut self) {
        for object in self.objects.iter() {
            self.free(*object);
        }
    }
}
