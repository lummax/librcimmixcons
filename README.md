librcimmixcons [![Build Status](https://travis-ci.org/lummax/librcimmixcons.svg?branch=master)](https://travis-ci.org/lummax/librcimmixcons)
==============

This is an implementation of the RCImmixCons garbage collector written in the
Rust Programming Language. For details please refer to:

- S. M. S. Blackburn and K. K. S. McKinley. Immix: a mark-region garbage
  collector with space efficiency, fast collection, and mutator performance.
  ACM SIGPLAN Notices, 43(6):22, May 2008.
- R. Shahriyar, S. M. Blackburn, and D. Frampton. Down for the count?  Getting
  reference counting back in the ring. ACM SIGPLAN Notices, 47(11):73, Jan.
  2013.
- R. Shahriyar, S. M. Blackburn, and K. S. McKinley. Fast conservative garbage
  collection. In Proceedings of the 2014 ACM International Conference on
  Object Oriented Programming Systems Languages & Applications - OOPSLA ’14,
  pages 121-139, New York, New York, USA, Oct. 2014. ACM Press.
- R. Shahriyar, S. M. Blackburn, X. Yang, and K. S. McKinley. Taking off the
  gloves with reference counting Immix. ACM SIGPLAN Notices, 48(10):93-110,
  Nov. 2013.

Status
------

This is not usable an the moment. Major TODOs are:

- TESTING
- Optimize `GCHeader` fields and use limited reference count bits
- Implement multi-threading (only single-thread applications are supported right now)
- Improve performance

What somewhat works (please refer to the integration tests in `tests/`):

- The C FFI and Valgrind integration
- Simple allocation and and overflow allocation
- Deferred coalesced reference counting collection
- Immix backup tracing (cycle) collection
- Opportunistic proactive and reactive defragmentation
- A simple free-list large-object-space with RC and MS collection
- Explicit adding of global (static) roots by the mutator program

And some features that would be nice:

- Pinning of objects by the mutator program
- Explicit setting kind of collection by the mutator program
- BlockInfo.{line_counter, object_map} as embedded data structures

Building
--------

You'll need [Rust](http://rust-lang.org/) and [cargo](http://crates.io)
installed. To build a development version use:

```
cargo build
```

To build without the debugging output and with some optimization please
use:

```
cargo build --release
```

The compiled shared-object file will be in `target/` or `target/release`

Using
-----

To include project in your C code include the header file `src/rcimmixcons.h`
and link against the shared-object file.

Large Object Space
------------------

The large object space (`LOS`) is currently implemented quite inefficiently.
To disable it build using the feature `no_large_object_space`.

```
cargo build --features "no_large_object_space"
cargo build --release --features "no_large_object_space"
```

Valgrind
--------

The tool `Memcheck` from `Valgrind` is supported using the macros
VALGRIND_MALLOCLIKE_BLOCK und VALGRIND_FREELIKE_BLOCK internally. Please build
using the feature `valgrind`. This is not enabled by default as it introduces
some overhead to determine freed objects after collection due to how Immix
works.

```
cargo build --features "valgrind"
cargo build --release --features "valgrind"
```
