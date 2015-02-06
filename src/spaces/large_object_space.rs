// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::collections::{HashSet, RingBuf};
use std::ptr;

use gc_object::{GCRTTI, GCObject, GCObjectRef};

pub struct LargeObjectSpace  {
    objects: HashSet<GCObjectRef>,
    new_objects: RingBuf<GCObjectRef>,
    free_buffer: RingBuf<GCObjectRef>,
    current_live_mark: bool,
}

impl LargeObjectSpace  {
    pub fn new() -> LargeObjectSpace {
        return LargeObjectSpace {
            objects: HashSet::new(),
            new_objects: RingBuf::new(),
            free_buffer: RingBuf::new(),
            current_live_mark: false,
        };
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.objects.contains(&object);
    }

    pub fn enqueue_free(&mut self, object: GCObjectRef) {
        self.free_buffer.push_back(object);
    }

    pub fn get_new_objects(&mut self) -> RingBuf<GCObjectRef> {
        return self.new_objects.drain().collect();
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
            self.new_objects.push_back(object);
            return Some(object);
        }
        return None;
    }

    pub fn proccess_free_buffer(&mut self) {
        debug!("Starting processing free_buffer size={} after RC collection",
               self.free_buffer.len());
        for object in self.free_buffer.drain() {
            debug!("Free object {:p} from RC collection", object);
            if self.objects.remove(&object) {
                unsafe{ libc::free(object as *mut libc::c_void); }
            }
        }
        debug!("Completed processing free_buffer after RC collection");
    }

    pub fn sweep(&mut self) {
        let next_live_mark = !self.current_live_mark;
        let is_marked = |o: &GCObjectRef| unsafe{ (**o).is_marked(next_live_mark) };
        debug!("Sweep LOS with next_live_mark={}", next_live_mark);
        let (marked, unmarked) : (Vec<_>, Vec<_>) = self.objects.drain().partition(is_marked);
        self.objects = marked.into_iter().collect();
        for object in unmarked {
            debug!("Free object {:p} in sweep", object);
            unsafe{ libc::free(object as *mut libc::c_void); }
        }
        debug!("Completed sweeping LOS after Immix collection");
    }
}

impl Drop for LargeObjectSpace {
    fn drop(&mut self) {
        for object in self.objects.iter() {
            unsafe{ libc::free(*object as *mut libc::c_void); }
        }
    }
}
