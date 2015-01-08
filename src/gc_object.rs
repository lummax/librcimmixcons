// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::mem;

use constants::LINE_SIZE;

#[repr(C)]
#[derive(PartialEq)]
#[allow(missing_copy_implementations)]
pub struct GCHeader {
    reference_count: libc::size_t,
    spans_lines: bool,
    forwarded: bool,
    logged: bool,
    marked: bool,
    pinned: bool,
    new: bool,
}

#[repr(C)]
#[allow(missing_copy_implementations)]
pub struct GCRTTI {
    object_size: libc::size_t,
    variables: libc::size_t,
}

#[repr(C)]
#[derive(PartialEq)]
#[allow(raw_pointer_derive)]
pub struct GCObject {
    header: GCHeader,
    rtti: *const GCRTTI
}

pub type GCObjectRef = *mut GCObject;

impl GCRTTI {
    pub fn new(object_size: uint, variables: uint) -> GCRTTI {
        return GCRTTI {
            object_size: object_size as libc::size_t,
            variables: variables as libc::size_t,
        };
    }

    pub fn object_size(&self) -> uint {
        return self.object_size as uint;
    }

    pub fn variables(&self) -> uint {
        return self.variables as uint;
    }
}

impl GCObject {
    pub fn new(rtti: *const GCRTTI, mark: bool) -> GCObject {
        debug!("GCobject::new(rtti={}, mark={}", rtti, mark);
        let size = unsafe{ (*rtti).object_size() };
        return GCObject {
            header: GCHeader {
                reference_count: 0,
                spans_lines: size > LINE_SIZE,
                forwarded: false,
                logged: false,
                marked: mark,
                pinned: false,
                new: true,
            },
            rtti: rtti,
        }
    }

    pub fn set_logged(&mut self, new: bool) -> bool {
        debug!("Set object {:p} logged={}", self, new);
        let logged = self.header.logged;
        self.header.logged = new;
        return logged;
    }

    pub fn set_marked(&mut self, next: bool) -> bool {
        debug!("Set object {:p} marked={}", self, next);
        let marked = self.header.marked;
        self.header.marked = next;
        return marked == next;
    }

    pub fn is_marked(&self, next: bool) -> bool {
        return self.header.marked == next;
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        debug!("Set object {:p} pinned={}", self, pinned);
        self.header.pinned = pinned;
    }

    pub fn is_pinned(&self) -> bool {
        return self.header.pinned;
    }

    pub fn set_forwarded(&mut self, new: GCObjectRef) {
        debug!("Set object {:p} forwarded to {}", self, new);
        self.header.forwarded = true;
        self.rtti = new as *const GCRTTI;
    }

    pub fn is_forwarded(&self) -> Option<GCObjectRef> {
        if self.header.forwarded {
            return Some(self.rtti as GCObjectRef);
        }
        return None;
    }

    pub fn spans_lines(&self) -> bool {
        return self.header.spans_lines;
    }

    pub fn object_size(&self) -> uint {
        return unsafe{ (*self.rtti).object_size() };
    }

    pub fn decrement(&mut self) -> bool {
        if self.header.reference_count == 0 {
            return false;
        }
        self.header.reference_count -= 1;
        debug!("Decrement object {:p} to {}", self, self.header.reference_count);
        return self.header.reference_count == 0;
    }

    pub fn increment(&mut self) -> bool {
        self.header.reference_count += 1;
        debug!("Increment object {:p} to {} (new={})", self,
            self.header.reference_count, self.header.new);
        if self.header.new {
            self.header.new = false;
            return true;
        }
        return false;
    }

    pub fn set_child(&mut self, num: uint, child: GCObjectRef) {
        unsafe {
            let base: *mut GCObjectRef = mem::transmute(&self.rtti);
            let address = base.offset((num + 1) as int);
            *address = child;
        }
    }

    pub fn children(&mut self) -> Vec<GCObjectRef> {
        let base: *const GCObjectRef = unsafe{ mem::transmute(&self.rtti) };
        let variables = unsafe{ (*self.rtti).variables() };
        debug!("Requested children for object: {:p} (rtti: {}, count: {})",
               self, self.rtti, variables);
        return range(1, variables + 1).map(|i| unsafe{ *base.offset(i as int) })
                                      .collect();
    }
}
