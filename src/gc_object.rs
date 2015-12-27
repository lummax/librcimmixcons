// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

extern crate libc;

use std::mem;
use std::ops::Range;

use constants::LINE_SIZE;

/// Structs that comprise the structure of an object as needed by the garbage
/// collector.
///
/// Include the `GCObject` struct as the first member in your object struct.
///
/// _Note:_ Only allocate and initialize the `GCRTTI`. `RCImmixCons.allocate()`
/// will return an initialized `GCObject` with a valid `GCHeader`.

/// The `GCHeader` contains field for the garbage collector algorithms.
#[repr(C)]
#[derive(PartialEq)]
#[allow(missing_copy_implementations)]
pub struct GCHeader {
    /// How many objects point to this object.
    reference_count: libc::size_t,

    /// If this object is greater than `LINE_SIZE`.
    spans_lines: bool,

    /// If the object at this address was forwarded somewhere else.
    forwarded: bool,

    /// If this object was pushed on the `modBuffer` in `RCCollector`.
    logged: bool,

    /// If this object was already visited by the tracing collector.
    ///
    /// _Note_: true/false do not mean marked/unmarked. The tracing collector
    /// will flip the meaning of the value for every collection cycle. See
    /// `Spaces.current_live_mark`.
    marked: bool,

    /// If this object must not be evacuated (moved) by the collector.
    pinned: bool,

    /// If this object was never touched by the collectors.
    new: bool,
}

/// The `GCRTTI` contains runtime type information about an object for the
/// garbage collector.
#[repr(C)]
#[allow(missing_copy_implementations)]
pub struct GCRTTI {
    /// The objects size in bytes.
    object_size: libc::size_t,

    /// How many pointers to other objects does this object contain.
    members: libc::size_t,
}

/// The `GCObject` is the base struct for every object managed by the garbage
/// collector.
///
/// Please include this as the first member in your object structs. The
/// members of this object _must_ be a contiguous array of `GCobject` pointers
/// of size `rtti.members`.
#[repr(C)]
#[derive(PartialEq)]
pub struct GCObject {
    /// The `GCHeader` for this object. This is initialized by the allocation
    /// routine.
    header: GCHeader,

    /// A pointer to the objects runtime type information struct.
    rtti: *const GCRTTI
}

/// A type alias for the mutable `GCObject` pointer.
pub type GCObjectRef = *mut GCObject;

impl GCRTTI {
    /// Create a new `GCRTTI` for an object with `object_size` bytes and
    /// `members` members.
    pub fn new(object_size: usize, members: usize) -> GCRTTI {
        GCRTTI {
            object_size: object_size as libc::size_t,
            members: members as libc::size_t,
        }
    }

    /// Return the objects size in bytes.
    pub fn object_size(&self) -> usize {
        self.object_size as usize
    }

    /// Return the number of members.
    pub fn members(&self) -> usize {
        self.members as usize
    }
}

impl GCObject {
    /// Create a new `GCObject` with `rtti` as the runtime typeinformation
    /// struct pointer and the current `mark` value.
    pub fn new(rtti: *const GCRTTI, mark: bool) -> GCObject {
        debug!("GCobject::new(rtti={:p}, mark={})", rtti, mark);
        let size = unsafe{ (*rtti).object_size() };
        GCObject {
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

    /// Set the `logged` state and return the previous value.
    pub fn set_logged(&mut self, new: bool) -> bool {
        debug!("Set object {:p} logged={}", self, new);
        let logged = self.header.logged;
        self.header.logged = new;
        logged
    }

    /// Set the `marked` state and return if the state has not
    /// changed.
    pub fn set_marked(&mut self, next: bool) -> bool {
        debug!("Set object {:p} marked={}", self, next);
        let marked = self.header.marked;
        self.header.marked = next;
        marked == next
    }

    /// Return if this object is currently marked with `next`.
    pub fn is_marked(&self, next: bool) -> bool {
        self.header.marked == next
    }

    /// Set the `pinned` state for this object.
    pub fn set_pinned(&mut self, pinned: bool) {
        debug!("Set object {:p} pinned={}", self, pinned);
        self.header.pinned = pinned;
    }

    /// Return if this object is pinned.
    pub fn is_pinned(&self) -> bool {
        self.header.pinned
    }

    /// Set the `forwarded` state and install a forewarding pointer to `new`.
    pub fn set_forwarded(&mut self, new: GCObjectRef) {
        debug!("Set object {:p} forwarded to {:p}", self, new);
        self.header.forwarded = true;
        self.rtti = new as *const GCRTTI;
    }

    /// Return a pointer to the forwarded object if this object was forwarded,
    /// otherwise `None`.
    pub fn is_forwarded(&self) -> Option<GCObjectRef> {
        if self.header.forwarded {
            Some(self.rtti as GCObjectRef)
        } else {
            None
        }
    }

    /// Returns if this object spans lines (is greater than `LINE_SIZE`
    /// bytes).
    pub fn spans_lines(&self) -> bool {
        self.header.spans_lines
    }

    /// Return the objects size in bytes.
    ///
    /// This rounds the size stored in the `GCRTTI` struct up to the
    /// target_pointer_width.
    #[cfg(target_pointer_width = "32")]
    pub fn object_size(&self) -> usize {
        let size = unsafe{ (*self.rtti).object_size() };
        return size + (size % 4)
    }

    /// Return the objects size in bytes.
    ///
    /// This rounds the size stored in the `GCRTTI` struct up to the
    /// target_pointer_width.
    #[cfg(target_pointer_width = "64")]
    pub fn object_size(&self) -> usize {
        let size = unsafe{ (*self.rtti).object_size() };
        size + (size % 8)
    }

    /// Decrement the referece counter and return true if the reference count
    /// is zero.
    ///
    /// This will not decrement the reference count if it is already zero.
    pub fn decrement(&mut self) -> bool {
        if self.header.reference_count == 0 {
            return false;
        }
        self.header.reference_count -= 1;
        debug!("Decrement object {:p} to {}", self, self.header.reference_count);
        self.header.reference_count == 0
    }

    /// Increment the reference count, set the `new` state to `false` and
    /// return the previous `new` state.
    pub fn increment(&mut self) -> bool {
        self.header.reference_count += 1;
        debug!("Increment object {:p} to {} (new={})", self,
            self.header.reference_count, self.header.new);
        if self.header.new {
            self.header.new = false;
            true
        } else {
            false
        }
    }

    /// Set the member at position `num` in the member array to `member`.
    pub fn set_member(&mut self, num: usize, member: GCObjectRef) {
        unsafe {
            let base: *mut GCObjectRef = mem::transmute(&self.rtti);
            let address = base.offset((num + 1) as isize);
            *address = member;
        }
    }

    /// Return an iterator of all the members of this object that are not null.
    ///
    /// The members of an objects are the `GCRTTI.members` pointers after the
    /// `GCHeader.rtti` pointer in the `GCObject`.
    pub fn children(&mut self) -> GCObjectRefIter {
        let base: *const GCObjectRef = unsafe{ mem::transmute(&self.rtti) };
        let members = unsafe{ (*self.rtti).members() };
        debug!("Requested children for object: {:p} (rtti: {:p}, count: {})",
               self, self.rtti, members);
        GCObjectRefIter::iter(1..(members + 1), base)
    }
}

/// An `Iterator` over a `range` of `GCObjectRef`s starting at `base`.
pub struct GCObjectRefIter {
    range: Range<usize>,
    base: *const GCObjectRef,
}

impl GCObjectRefIter {
    /// Create a new `GCObjectRefIter` over `range`, starting at `base`.
    pub fn iter(range: Range<usize>, base: *const GCObjectRef) -> GCObjectRefIter {
        GCObjectRefIter {
            range: range,
            base: base,
        }
    }
}

impl Iterator for GCObjectRefIter {
    type Item = GCObjectRef;

    fn next(&mut self) -> Option<GCObjectRef> {
        for num in self.range.by_ref() {
            let child = unsafe{ *self.base.offset(num as isize) };
            if !child.is_null() {
                return Some(child);
            } else { continue }
        }
        None
    }
}
