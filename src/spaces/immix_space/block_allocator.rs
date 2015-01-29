// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::{ptr, os};

use spaces::immix_space::block_info::BlockInfo;

use constants::{BLOCK_SIZE, HEAP_SIZE};
use gc_object::GCObjectRef;

pub struct BlockAllocator {
    mmap: os::MemoryMap,
    data: *mut u8,
    data_bound: *mut u8,
    free_blocks: Vec<*mut BlockInfo>,
}

impl BlockAllocator {
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

    pub fn get_block(&mut self) -> Option<*mut BlockInfo> {
        return self.free_blocks.pop().or_else(|| self.build_next_block());
    }

    pub fn return_block(&mut self, block: *mut BlockInfo) {
        debug!("Returned block {:p}", block);
        self.free_blocks.push(block);
    }

    pub fn total_blocks(&self) -> usize {
        return HEAP_SIZE / BLOCK_SIZE;
    }

    pub fn available_blocks(&self) -> usize {
        return (((self.data_bound as usize) - (self.data as usize)) % BLOCK_SIZE)
            + self.free_blocks.len();
    }

    pub fn is_in_space(&self, object: GCObjectRef) -> bool {
        return self.mmap.data() < (object as *mut u8)
            && (object as *mut u8) < self.data_bound;
    }
}

impl BlockAllocator{
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
