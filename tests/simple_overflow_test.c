// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include "../src/rcimmixcons.h"
#include <stdio.h>
#include <assert.h>

typedef struct {
    GCObject object;
} SmallObject;

static GCRTTI smallObjectRTTI = {sizeof(SmallObject), 0};

typedef struct {
    GCObject object;
    int data[512];
} LargeObject;

static GCRTTI largeObjectRTTI = {sizeof(LargeObject), 0};


int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    for (int times = 0; times < 3; times++) {
        SmallObject* small_object = (SmallObject*) rcx_allocate(collector, &smallObjectRTTI);
        assert(small_object != NULL);
        printf("(mutator) Address of small_object: %p\n", small_object);
        fflush(stdout);
        LargeObject* large_object = (LargeObject*) rcx_allocate(collector, &largeObjectRTTI);
        assert(large_object != NULL);
        printf("(mutator) Address of large_object: %p\n", large_object);
        fflush(stdout);
    }
    rcx_collect(collector);
    rcx_destroy(collector);
    return 0;
}
