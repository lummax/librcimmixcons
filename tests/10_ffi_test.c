// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>
#include <assert.h>

static GCRTTI dummyObjectRTTI = {128, 0};

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    GCObject* object = rcx_allocate(collector, &dummyObjectRTTI);
    assert(object != NULL);
    printf("(mutator) Address of object: %p\n", object);
    fflush(stdout);
    rcx_collect(collector, 0, 0);
    assert(object != NULL);
    rcx_write_barrier(collector, object);
    assert(object != NULL);
    rcx_destroy(collector);
    return 0;
}