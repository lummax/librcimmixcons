// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>
#include <assert.h>

typedef struct CicleObject {
    GCObject object;
    struct CicleObject* next;
    int data[16];
} CicleObject;

static GCRTTI CircleObjectRtti = {sizeof(CicleObject), 1};

typedef struct {
    GCObject object;
    CicleObject* attr_a;
    CicleObject* attr_b;
} CompositeObject;

static GCRTTI compositeObjectRTTI = {sizeof(CompositeObject), 2};

CicleObject* build_circle_object(RCImmixCons* collector) {
    CicleObject* new_cicle_object_a = (CicleObject*) rcx_allocate(collector, &CircleObjectRtti);
    assert(new_cicle_object_a != NULL);
    CicleObject* new_cicle_object_b = (CicleObject*) rcx_allocate(collector, &CircleObjectRtti);
    assert(new_cicle_object_b != NULL);
    CicleObject* new_cicle_object_c = (CicleObject*) rcx_allocate(collector, &CircleObjectRtti);
    assert(new_cicle_object_c != NULL);
    printf("(mutator) Address of new_cicle_object_a: %p\n", new_cicle_object_a);
    printf("(mutator) Address of new_cicle_object_b: %p\n", new_cicle_object_b);
    printf("(mutator) Address of new_cicle_object_c: %p\n", new_cicle_object_b);
    fflush(stdout);
    new_cicle_object_a->next = new_cicle_object_b;
    new_cicle_object_b->next = new_cicle_object_c;
    new_cicle_object_c->next = new_cicle_object_a;

    return new_cicle_object_a;
}

void change_object(RCImmixCons* collector, CompositeObject* object) {
    CicleObject* new_circle_object_a = build_circle_object(collector);
    CicleObject* new_circle_object_b = build_circle_object(collector);
    object->attr_a = new_circle_object_a;
    object->attr_b = new_circle_object_b;
}

CompositeObject* build_object(RCImmixCons* collector) {
    CompositeObject* composite_object = (CompositeObject*) rcx_allocate(collector, &compositeObjectRTTI);
    assert(composite_object != NULL);
    change_object(collector, composite_object);
    printf("(mutator) Address of composite_object: %p\n", composite_object);
    fflush(stdout);
    return composite_object;
}

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    CompositeObject* composite_object = build_object(collector);
    rcx_collect(collector, 0, 1);
    assert(composite_object != NULL);
    rcx_write_barrier(collector, (GCObject*) composite_object);
    assert(composite_object != NULL);
    change_object(collector, composite_object);
    rcx_collect(collector, 0, 1);
    assert(composite_object != NULL);
    rcx_destroy(collector);
    return 0;
}
