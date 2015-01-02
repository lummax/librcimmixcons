// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>

typedef struct {
    GCObject object;
} SimpleObject;

typedef struct {
    GCObject object;
    SimpleObject* attr_a;
    SimpleObject* attr_b;
} CompositeObject;

CompositeObject* build_object(RCImmixCons* collector) {
    SimpleObject* simple_object_a = (SimpleObject*) rcx_allocate(collector, sizeof(SimpleObject), 0);
    SimpleObject* simple_object_b = (SimpleObject*) rcx_allocate(collector, sizeof(SimpleObject), 0);
    CompositeObject* composite_object = (CompositeObject*) rcx_allocate(collector, sizeof(CompositeObject), 2);
    printf("(mutator) Address of simple_object_a: %p\n", simple_object_a);
    printf("(mutator) Address of simple_object_b: %p\n", simple_object_b);
    printf("(mutator) Address of composite_object: %p\n", composite_object);
    composite_object->attr_a = simple_object_a;
    composite_object->attr_b = simple_object_b;
    return composite_object;
}

int main() {
    RCImmixCons* collector = rcx_create();
    CompositeObject* composite_object = build_object(collector);
    rcx_collect(collector);
    rcx_destroy(collector);
    return 0;
}
