// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#ifndef RCIMMIXCONS_H
#define RCIMMIXCONS_H

#include <stdint.h>
#include <stdlib.h>

typedef struct {
    size_t reference_count;
    uint8_t spans_lines;
    uint8_t forwarded;
    uint8_t logged;
    uint8_t marked;
    uint8_t new;
} GCHeader;

typedef struct {
    size_t object_size;
    size_t num_variables;
} GCRTTI;

typedef struct {
    GCHeader header;
    GCRTTI* rtti;
} GCObject;

typedef struct {} RCImmixCons;

RCImmixCons* rcx_create();
GCObject* rcx_allocate(RCImmixCons* collector, GCRTTI* rtti);
void rcx_collect(RCImmixCons* collector, uint8_t evacuation, uint8_t cycle_collect);
void rcx_write_barrier(RCImmixCons* collector, GCObject* object);
void rcx_destroy(RCImmixCons* collector);

#endif
