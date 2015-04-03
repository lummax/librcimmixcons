// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![allow(deprecated)]
use std::ptr;
use libc;

use spaces::immix_space::block_info::BlockInfo;

use constants::{BLOCK_SIZE, HEAP_SIZE, TOTAL_BLOCKS};
use gc_object::GCObjectRef;

/// A simple wrapper for a heap mmap.
struct MemoryMap{
    /// The pointer to the mmap'ed region.
    mmap: *mut libc::c_void,
}

impl MemoryMap {
    /// Create a new `MemoryMap` of `HEAP_SIZE + BLOCK_SIZE` to be aligned to
    /// `BLOCK_SIZE` boundaries.
    fn new() -> MemoryMap {
        let mmap = unsafe {
            libc::mmap(ptr::null_mut(), (HEAP_SIZE + BLOCK_SIZE) as libc::size_t,
                       libc::PROT_READ | libc::PROT_WRITE,
                      libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0)
        };

        if mmap == libc::MAP_FAILED {
            panic!("Failed to allocate the heap memory map");
        }

        return MemoryMap {
            mmap: mmap,
        }
    }

    /// Return a `BLOCK_SIZE` aligned pointer to the mmap'ed region.
    fn aligned(&self) -> *mut u8 {
        let offset = BLOCK_SIZE - (self.mmap as usize) % BLOCK_SIZE;
        return unsafe{ self.mmap.offset(offset as isize) } as *mut u8;
    }

    /// Return a pointer to the start of the mmap'ed region.
    fn start(&self) -> *mut u8 {
        return self.mmap as *mut u8;
    }

    /// Return a pointer to the end of the mmap'ed region.
    fn bound(&self) -> *mut u8 {
        return unsafe{ self.mmap.offset(HEAP_SIZE as isize) } as *mut u8;
    }
}

impl Drop for MemoryMap {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mmap, (HEAP_SIZE + BLOCK_SIZE) as libc::size_t);
        }
    }
}


/// The `BlockAllocator` is the global resource for blocks for the immix
/// space.
///
/// On initialization it will allocate a memory map of `HEAP_SIZE` and align
/// it to `BLOCK_SIZE`. During normal runtime it will allocate blocks on the
/// fly from this memory map and store returned blocks in a list.
///
/// Blocks from this `BlockAllocator` are always aligned to `BLOCK_SIZE`.
///
/// The list of returned free blocks is a stack. The `BlockAllocator` will
/// first exhaust the returned free blocks and then fall back to allocating
/// new blocks from the memory map. This means it will return recently
/// returned blocks first.
pub struct BlockAllocator {
    /// The memory map of `HEAP_SIZE`.
    mmap: MemoryMap,

    /// The pointer to the last allocated block.
    data: *mut u8,

    /// The upper bound of the memory map as a pointer.
    data_bound: *mut u8,

    /// A list of returned (free) blocks.
    free_blocks: Vec<*mut BlockInfo>,
}

impl BlockAllocator {
    /// Create a new `BlockAllocator`.
    ///
    /// This will `panic` if no memory map of size `HEAP_SIZE` can be
    /// allocared.
    pub fn new() -> BlockAllocator {
        let mmap = MemoryMap::new();
        let data = mmap.aligned();
        let bound = mmap.bound();
        debug_assert!((data as usize) % BLOCK_SIZE == 0,
            "Allocated mmap {:p} is not aligned (offset {})",
            data, (data as usize) % BLOCK_SIZE);
        return BlockAllocator {
            mmap: mmap,
            data: data,
            data_bound: bound,
            free_blocks: Vec::with_capacity(TOTAL_BLOCKS),
        };
    }

    /// Get a new block aligned to `BLOCK_SIZE`.
    pub fn get_block(&mut self) -> Option<*mut BlockInfo> {
        return self.free_blocks.pop().or_else(|| self.build_next_block());
    }

    /// Return a collection of blocks.
    pub fn return_blocks(&mut self, blocks: Vec<*mut BlockInfo>) {
        self.free_blocks.extend(blocks.into_iter());
    }

    /// Return the number of unallocated blocks.
    pub fn available_blocks(&self) -> usize {
        return (((self.data_bound as usize) - (self.data as usize)) % BLOCK_SIZE)
            + self.free_blocks.len();
    }

    /// Return if an address is within the bounds of the memory map.
    pub fn is_in_space(&self, object: GCObjectRef) -> bool {
        return self.mmap.start() < (object as *mut u8)
            && (object as *mut u8) < self.data_bound;
    }
}

impl BlockAllocator {
    /// Build a new block from the memory map.
    ///
    /// Returns `None` if the memory map is exhausted.
    fn build_next_block(&mut self) -> Option<*mut BlockInfo> {
        let block = unsafe{ self.data.offset(BLOCK_SIZE as isize) };
        if block < self.data_bound {
            self.data = block;
            debug_assert!((block as usize) % BLOCK_SIZE == 0,
                "Allocated block {:p} is not aligned (offset {})",
                block, (block as usize) % BLOCK_SIZE);
            unsafe{ ptr::write(block as *mut BlockInfo, BlockInfo::new()); }
            return Some(block as *mut BlockInfo);
        }
        return None;
    }
}
