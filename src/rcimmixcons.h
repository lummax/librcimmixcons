// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#ifndef RCIMMIXCONS_H
#define RCIMMIXCONS_H

#include <stdint.h>
#include <stdlib.h>

/// The `GCHeader` contains field for the garbage collector algorithms.
typedef struct {
    /// How many objects point to this object.
    size_t reference_count;

    /// If this object is greater than `LINE_SIZE`.
    uint8_t spans_lines;

    /// If the object at this address was forwarded somewhere else.
    uint8_t forwarded;

    /// If this object was pushed on the `modBuffer` in `RCCollector`.
    uint8_t logged;

    /// If this object was already visited by the tracing collector.
    uint8_t marked;

    /// If this object must not be evacuated (moved) by the collector.
    uint8_t pinned;

    /// If this object was never touched by the collectors.
    uint8_t new;
} GCHeader;

/// The `GCRTTI` contains runtime type information about an object for the
/// garbage collector.
typedef struct {
    /// The objects size in bytes.
    size_t object_size;

    /// How many pointers to other objects does this object contain.
    size_t num_members;
} GCRTTI;

/// The `GCObject` is the base struct for every object managed by the garbage
/// collector.
///
/// Please include this as the first member in your object structs. The
/// members of this object _must_ be a contiguous array of `GCobject` pointers
/// of size `rtti->members`.
typedef struct {
    /// The `GCHeader` for this object. This is initialized by the allocation
    /// routine.
    GCHeader header;

    /// A pointer to the objects runtime type information struct.
    GCRTTI* rtti;
} GCObject;

/// The `RCImmixCons` garbage collector.
///
/// This is the conservative reference counting garbage collector with the
/// immix heap partition schema.
///
/// The `rcx_allocate()` function will return a pointer to a `GCObject`.
/// Please see the documentation of `GCHeader`, `GCRTTI` and `GCObject` for
/// details.
///
/// Always call `rcx_write_barrier()` on an object before modifying its
/// members.
typedef struct {} RCImmixCons;

/// Create a new `RCImmixCons`.
RCImmixCons* rcx_create(void);

/// Allocate a new object described by the `rtti` or returns `NULL`.
///
/// This may trigger a garbage collection if the allocation was not
/// succussful. If there is still no memory to fullfill the allocation
/// request return `NULL`.
GCObject* rcx_allocate(RCImmixCons* collector, GCRTTI* rtti);

/// Trigger a garbage collection.
///
/// This will always run the referece counting collector. If `evacuation`
/// is set the collectors will try to evacuate. If `cycle_collect` is set
/// the immix tracing collector will be used.
void rcx_collect(RCImmixCons* collector, uint8_t evacuation, uint8_t cycle_collect);

/// Set an address to an object reference as static root.
///
/// Use this to mark global/static variables as roots. This is needed, if  the
/// pointer to a garbage collected object does not reside on the stack or in
/// any register.
void rcx_set_static_root(RCImmixCons* collector, void* address);

/// A write barrier for the given `object`.
///
/// Call this function before modifying the members of this object!
void rcx_write_barrier(RCImmixCons* collector, GCObject* object);

/// Destroy and cleanup the garbage collector.
void rcx_destroy(RCImmixCons* collector);

#endif
