// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::collections::{HashSet, RingBuf};
use std::ptr;

use gc_object::{GCRTTI, GCObject, GCObjectRef};

/// The large object space is used to allocate objects of `LARGE_OBJECT` bytes
/// size.
///
/// This space is a simple free-list allocator collected by the reference
/// counting collector (without proactive opportunistic evacuation) and a
/// mark-and-sweep integrated into the immix tracing collector.
pub struct LargeObjectSpace  {
    /// A set of addresses that are valid objects. Needed for the conservative
    /// part.
    objects: HashSet<GCObjectRef>,

    /// Objects in this block that were never touched by the garbage
    /// collector.
    new_objects: RingBuf<GCObjectRef>,

    /// A buffer of elements to be freed after the RC collection phase.
    free_buffer: RingBuf<GCObjectRef>,

    /// The current live mark for new objects. See `Spaces.current_live_mark`.
    current_live_mark: bool,
}

impl LargeObjectSpace  {
    /// Create a new `LargeObjectSpace`.
    pub fn new() -> LargeObjectSpace {
        return LargeObjectSpace {
            objects: HashSet::new(),
            new_objects: RingBuf::new(),
            free_buffer: RingBuf::new(),
            current_live_mark: false,
        };
    }

    /// Return if the object an the address is a valid object within the large
    /// object space.
    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.objects.contains(&object);
    }

    /// Enqueue an object to be freed after the RC collection phase.
    pub fn enqueue_free(&mut self, object: GCObjectRef) {
        self.free_buffer.push_back(object);
    }

    /// Get the new objects of the large object space.
    pub fn get_new_objects(&mut self) -> RingBuf<GCObjectRef> {
        return self.new_objects.drain().collect();
    }

    /// Set the current live mark to `current_live_mark`.
    pub fn set_current_live_mark(&mut self, current_live_mark: bool) {
        self.current_live_mark = current_live_mark;
    }

    /// Allocate an object of `size` bytes or return `None` if the allocation
    /// failed.
    ///
    /// This object is initialized and ready to use.
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

    /// Free the objects in the free buffer.
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

    /// Sweep the objects within the large object space and free those that
    /// were not marked with the current live mark by the tracing collector.
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
