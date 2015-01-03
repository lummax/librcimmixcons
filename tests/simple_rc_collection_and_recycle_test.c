// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>

typedef struct {
    GCObject object;
    int data[512];
} LargeObject;

static GCRTTI largeObjectRTTI = {sizeof(LargeObject), 0};

typedef struct {
    GCObject object;
    LargeObject* attr_a;
    LargeObject* attr_b;
} CompositeObject;

static GCRTTI compositeObjectRTTI = {sizeof(CompositeObject), 2};

void change_object(RCImmixCons* collector, CompositeObject* object) {
    LargeObject* new_large_object_a = (LargeObject*) rcx_allocate(collector, &largeObjectRTTI);
    LargeObject* new_large_object_b = (LargeObject*) rcx_allocate(collector, &largeObjectRTTI);
    printf("(mutator) Address of new_large_object_a: %p\n", new_large_object_a);
    printf("(mutator) Address of new_large_object_b: %p\n", new_large_object_b);
    fflush(stdout);
    object->attr_a = new_large_object_a;
    object->attr_b = new_large_object_b;
}

CompositeObject* build_object(RCImmixCons* collector) {
    CompositeObject* composite_object = (CompositeObject*) rcx_allocate(collector, &compositeObjectRTTI);
    change_object(collector, composite_object);
    printf("(mutator) Address of composite_object: %p\n", composite_object);
    fflush(stdout);
    return composite_object;
}

int main() {
    RCImmixCons* collector = rcx_create();
    CompositeObject* composite_object = build_object(collector);
    rcx_collect(collector);
    for(int times = 0; times < 3; times++) {
        rcx_write_barrier(collector, (GCObject*) composite_object);
        change_object(collector, composite_object);
        rcx_collect(collector);
    }
    rcx_destroy(collector);
    return 0;
}
