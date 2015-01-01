extern crate libc;

use std::num::Int;
use std::ptr;
use std::mem;

use constants::LINE_SIZE;

#[repr(C)]
pub struct GCHeader {
    object_size: libc::size_t,
    variables: libc::size_t,
    reference_count: libc::size_t,
    spans_lines: bool,
    forwarded: bool,
    logged: bool,
    marked: bool,
    new: bool,
}

#[repr(C)]
pub struct GCObject {
    header: GCHeader,
    vmt_pointer: *mut u8,
}

impl GCObject {
    pub fn new(size: uint, variables: uint) -> GCObject {
        return GCObject {
            header: GCHeader {
                object_size: size as libc::size_t,
                variables: variables as libc::size_t,
                reference_count: 0,
                spans_lines: size > LINE_SIZE,
                forwarded: false,
                logged: false,
                marked: false,
                new: true,
            },
            vmt_pointer: ptr::null_mut(),
        }
    }

    pub fn object_size(&self) -> uint {
        return self.header.object_size as uint;
    }

    pub fn children(&mut self) -> Vec<*mut GCObject> {
        let base: *const *mut GCObject = unsafe{ mem::transmute(&self.vmt_pointer) };
        return range(1, self.header.variables + 1)
               .map(|i| unsafe{ *base.offset(i as int) })
               .collect();
    }

    pub fn decrement(&mut self) -> bool {
        self.header.reference_count = Int::saturating_sub(self.header.reference_count, 1);
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

    pub fn set_logged(&mut self, new: bool) -> bool {
        debug!("Set object {:p} logged={}", self, new);
        let logged = self.header.logged;
        self.header.logged = new;
        return logged;
    }

    pub fn spans_lines(&self) -> bool {
        return self.header.spans_lines;
    }
}

