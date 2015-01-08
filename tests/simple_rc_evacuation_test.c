// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>
#include <assert.h>

typedef struct {
    GCObject object;
    int data[10];
} SimpleObject;

static GCRTTI simpleObjectRTTI = {sizeof(SimpleObject), 0};

typedef struct {
    GCObject object;
    SimpleObject* attributes[15];
    int data[10];
} CompositeObject;

static GCRTTI compositeObjectRTTI = {sizeof(CompositeObject), 15};

CompositeObject* build_object(RCImmixCons* collector) {
    CompositeObject* composite_object = (CompositeObject*) rcx_allocate(collector, &compositeObjectRTTI);
    assert(composite_object != NULL);
    printf("(mutator) Address of composite_object: %p\n", composite_object);
    fflush(stdout);
    for (int i = 1; i < 31; i++) {
        SimpleObject* simpleObject = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
        assert(simpleObject != NULL);
        if (i & 2) {
            int index = ((i + 1) / 2) - 1;
            composite_object->attributes[index] = simpleObject;
            printf("(mutator) Set attribut %d to %p\n", index, simpleObject);
        } else { printf("(mutator) Address of simpleObject: %p\n", simpleObject); }
        fflush(stdout);
    }
    return composite_object;
}

void exchange_attributes(CompositeObject* object_a, CompositeObject* object_b) {
    for (int i = 0; i < 5; i++) {
        object_a->attributes[i] = object_b->attributes[i];
    }
    for (int i = 5; i < 10; i++) {
        object_b->attributes[i] = object_a->attributes[i];
    }
}

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);

    for (int times = 0; times < 3; times++) { build_object(collector); }
    CompositeObject* composite_object_a = build_object(collector);
    rcx_collect(collector);
    assert(composite_object_a != NULL);

    CompositeObject* composite_object_b = build_object(collector);
    rcx_write_barrier(collector, (GCObject*) composite_object_a);
    assert(composite_object_a != NULL);
    exchange_attributes(composite_object_a, composite_object_b);
    rcx_collect(collector);
    assert(composite_object_a != NULL);
    assert(composite_object_b != NULL);

    rcx_destroy(collector);
    return 0;
}
