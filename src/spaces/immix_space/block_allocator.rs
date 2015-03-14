// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

#![allow(deprecated)]
use std::{ptr, os};

use spaces::immix_space::block_info::BlockInfo;

use constants::{BLOCK_SIZE, HEAP_SIZE};
use gc_object::GCObjectRef;

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
    mmap: os::MemoryMap,

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
        let mmap = os::MemoryMap::new(HEAP_SIZE + BLOCK_SIZE,
                                      &[os::MapOption::MapReadable,
                                        os::MapOption::MapWritable]).unwrap();
        let data = unsafe{ mmap.data().offset((BLOCK_SIZE - (mmap.data() as usize) % BLOCK_SIZE) as isize) };
        let data_bound = unsafe{ mmap.data().offset(mmap.len() as isize) };
        debug!("Allocated heap {:p} of size {}, usable range: {:p} - {:p} (size {}, {} blocks)",
                mmap.data(), mmap.len(), data, data_bound,
                (data_bound as usize) - (data  as usize),
                ((data_bound as usize) - (data  as usize)) / BLOCK_SIZE);
        debug_assert!((data as usize) % BLOCK_SIZE == 0,
            "Allocated mmap {:p} is not aligned (offset {})",
            data, (data as usize) % BLOCK_SIZE);
        return BlockAllocator {
            mmap: mmap,
            data: data,
            data_bound: data_bound,
            free_blocks: Vec::with_capacity(HEAP_SIZE / BLOCK_SIZE),
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

    /// Return the total number of possible blocks.
    pub fn total_blocks(&self) -> usize {
        return HEAP_SIZE / BLOCK_SIZE;
    }

    /// Return the number of unallocated blocks.
    pub fn available_blocks(&self) -> usize {
        return (((self.data_bound as usize) - (self.data as usize)) % BLOCK_SIZE)
            + self.free_blocks.len();
    }

    /// Return if an address is within the bounds of the memory map.
    pub fn is_in_space(&self, object: GCObjectRef) -> bool {
        return self.mmap.data() < (object as *mut u8)
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
