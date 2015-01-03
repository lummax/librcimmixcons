librcimmixcons [![Build Status](https://travis-ci.org/lummax/librcimimxcons.svg?branch=master)](https://travis-ci.org/lummax/librcimimxcons)
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
- Implement opportunistic proactive and reactive defragmentation
- Implement a Large-Object-Space
- Optimize `GCHeader` fields and use limited reference count bits
- Implement multi-threading (only single-thread applications are supported right now)
- Improve performance

Building
--------

You'll need [Rust](http://rust-lang.org/) and [cargo](http://crates.io)
installed. To build a development version use:

```
cargo build
```

To build without the debugging output and a little optimization (-O1) please
use:

```
cargo build --release
```

The compiled shared-object file will be in `target/` or `target/release`

Using
-----

To include project in your C code include the header file `src/rcimmixcons.h`
and link against the shared-object file.
