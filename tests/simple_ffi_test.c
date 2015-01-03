// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>

static GCRTTI dummyObjectRTTI = {128, 0};

int main() {
    RCImmixCons* collector = rcx_create();
    GCObject* object = rcx_allocate(collector, &dummyObjectRTTI);
    printf("(mutator) Address of object: %p\n", object);
    fflush(stdout);
    rcx_collect(collector);
    rcx_write_barrier(collector, object);
    rcx_destroy(collector);
    return 0;
}
