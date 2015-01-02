#include "../src/rcimmixcons.h"
#include <stdio.h>

typedef struct CicleObject {
    GCObject object;
    struct CicleObject* next;
    int data[16];
} CicleObject;


void build_object(RCImmixCons* collector) {
    CicleObject* new_cicle_object_a = (CicleObject*) rcx_allocate(collector, sizeof(CicleObject), 1);
    CicleObject* new_cicle_object_b = (CicleObject*) rcx_allocate(collector, sizeof(CicleObject), 1);
    CicleObject* new_cicle_object_c = (CicleObject*) rcx_allocate(collector, sizeof(CicleObject), 1);
    printf("(mutator) Address of new_cicle_object_a: %p\n", new_cicle_object_a);
    printf("(mutator) Address of new_cicle_object_b: %p\n", new_cicle_object_b);
    printf("(mutator) Address of new_cicle_object_c: %p\n", new_cicle_object_b);
    new_cicle_object_a->next = new_cicle_object_b;
    new_cicle_object_b->next = new_cicle_object_c;
    new_cicle_object_c->next = new_cicle_object_a;
}

int main() {
    RCImmixCons* collector = rcx_create();
    for(int times = 0; times < 3; times++) {
        build_object(collector);
    }
    rcx_collect(collector);
    for(int times = 0; times < 3; times++) {
        build_object(collector);
    }
    rcx_collect(collector);
    rcx_destroy(collector);
    return 0;
}
