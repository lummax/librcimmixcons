// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>
#include <assert.h>

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

CompositeObject* build_object(RCImmixCons* collector) {
    SimpleObject* simple_object_a = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
    assert(simple_object_a != NULL);
    SimpleObject* simple_object_b = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
    assert(simple_object_b != NULL);
    CompositeObject* composite_object = (CompositeObject*) rcx_allocate(collector, &compositeObjectRTTI);
    assert(composite_object != NULL);
    printf("(mutator) Address of simple_object_a: %p\n", simple_object_a);
    printf("(mutator) Address of simple_object_b: %p\n", simple_object_b);
    printf("(mutator) Address of composite_object: %p\n", composite_object);
    fflush(stdout);
    composite_object->attr_a = simple_object_a;
    composite_object->attr_b = simple_object_b;
    return composite_object;
}

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    CompositeObject* composite_object = build_object(collector);
    rcx_collect(collector);
    assert(composite_object != NULL);
    rcx_destroy(collector);
    return 0;
}
