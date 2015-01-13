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

void build_object(RCImmixCons* collector) {
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
}

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    for(int times = 0; times < 3; times++) {
        build_object(collector);
    }
    rcx_collect(collector, 0, 1);
    for(int times = 0; times < 3; times++) {
        build_object(collector);
    }
    rcx_collect(collector, 0, 1);
    rcx_destroy(collector);
    return 0;
}
