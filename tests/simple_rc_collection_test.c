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

void change_object(RCImmixCons* collector, CompositeObject* object) {
    SimpleObject* new_simple_object_a = (SimpleObject*) rcx_allocate(collector, sizeof(SimpleObject), 0);
    SimpleObject* new_simple_object_b = (SimpleObject*) rcx_allocate(collector, sizeof(SimpleObject), 0);
    printf("(mutator) Address of new_simple_object_a: %p\n", new_simple_object_a);
    printf("(mutator) Address of new_simple_object_b: %p\n", new_simple_object_b);
    object->attr_a = new_simple_object_a;
    object->attr_b = new_simple_object_b;
}

CompositeObject* build_object(RCImmixCons* collector) {
    CompositeObject* composite_object = (CompositeObject*) rcx_allocate(collector, sizeof(CompositeObject), 2);
    change_object(collector, composite_object);
    printf("(mutator) Address of composite_object: %p\n", composite_object);
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