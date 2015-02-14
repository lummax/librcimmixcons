// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>
#include <assert.h>

typedef struct {
    GCObject object;
    int counter;
} SimpleObject;

static GCRTTI simpleObjectRTTI = {sizeof(SimpleObject), 0};

typedef struct {
    GCObject object;
    SimpleObject* attr_a;
    SimpleObject* attr_b;
} CompositeObject;

static GCRTTI compositeObjectRTTI = {sizeof(CompositeObject), 2};

static CompositeObject* composite_object = NULL;

CompositeObject* build_object(RCImmixCons* collector) {
    SimpleObject* simple_object_a = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
    assert(simple_object_a != NULL);
    SimpleObject* simple_object_b = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
    assert(simple_object_b != NULL);
    CompositeObject* composite_local = (CompositeObject*) rcx_allocate(collector, &compositeObjectRTTI);
    assert(composite_local != NULL);
    printf("(mutator) Address of simple_object_a: %p\n", simple_object_a);
    printf("(mutator) Address of simple_object_b: %p\n", simple_object_b);
    printf("(mutator) Address of composite_object: %p\n", composite_local);
    fflush(stdout);
    simple_object_a->counter = 0;
    simple_object_b->counter = 0;
    composite_local->attr_a = simple_object_a;
    composite_local->attr_b = simple_object_b;
    return composite_local;
}

void do_work(RCImmixCons* collector) {
    CompositeObject* old = composite_object;
    composite_object = build_object(collector);
    assert(old != composite_object);
    composite_object->attr_a->counter = old->attr_a->counter + 1;
    composite_object->attr_b->counter = old->attr_b->counter + 1;
    assert(composite_object->attr_a->counter > 0);
    assert(composite_object->attr_b->counter > 0);
}

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    rcx_set_static_root(collector, &composite_object);
    composite_object = build_object(collector);
    assert(composite_object != NULL);
    rcx_collect(collector, 1, 1);
    for (int i = 0; i < 3; i++) {
        do_work(collector);
        rcx_collect(collector, 1, 1);

        rcx_allocate(collector, &simpleObjectRTTI);
        rcx_collect(collector, 1, 1);
    }
    printf("(mutator) Value of attr_a: %d\n", composite_object->attr_a->counter);
    fflush(stdout);
    assert(composite_object->attr_a->counter == 3);
    printf("(mutator) Value of attr_b: %d\n", composite_object->attr_b->counter);
    fflush(stdout);
    assert(composite_object->attr_b->counter == 3);
    rcx_destroy(collector);
    return 0;
}
