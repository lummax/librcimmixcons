// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>

typedef struct {
    GCObject object;
} SimpleObject;

static GCRTTI simpleObjectRTTI = {sizeof(SimpleObject), 0};

typedef struct {
    GCObject object;
    SimpleObject* attr_a;
    SimpleObject* attr_b;
} CompositeObject;

static GCRTTI compositeObjectRTTI = {sizeof(CompositeObject), 2};

void change_object(RCImmixCons* collector, CompositeObject* object) {
    SimpleObject* new_simple_object_a = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
    SimpleObject* new_simple_object_b = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
    printf("(mutator) Address of new_simple_object_a: %p\n", new_simple_object_a);
    printf("(mutator) Address of new_simple_object_b: %p\n", new_simple_object_b);
    fflush(stdout);
    object->attr_a = new_simple_object_a;
    object->attr_b = new_simple_object_b;
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
    rcx_write_barrier(collector, (GCObject*) composite_object);
    change_object(collector, composite_object);
    rcx_collect(collector);
    rcx_destroy(collector);
    return 0;
}
