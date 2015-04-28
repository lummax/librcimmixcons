// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#include <rcimmixcons.h>
#include <stdio.h>
#include <assert.h>

typedef struct {
    GCObject object;
    int data[32];
} SimpleObject;

static GCRTTI simpleObjectRTTI = {sizeof(SimpleObject), 0};

int main() {
    RCImmixCons* collector = rcx_create();
    assert(collector != NULL);
    for(int times = 0; times < 3; times++) {
        for(int tim = 0; tim < 256; tim++) {
            SimpleObject* object = (SimpleObject*) rcx_allocate(collector, &simpleObjectRTTI);
            printf("(mutator) Address of object: %p\n", object);
            fflush(stdout);
            assert(object != NULL);
        }
        rcx_collect(collector, 0, 0);
    }
    rcx_destroy(collector);
    return 0;
}
