// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::{os, ptr};

use block_info::BlockInfo;
use constants::BLOCK_SIZE;

pub struct BlockAllocator {
    free_blocks: Vec<os::MemoryMap>,
}

impl BlockAllocator {
    pub fn new() -> BlockAllocator {
        return BlockAllocator {
            free_blocks: Vec::new(),
        };
    }

    pub fn get_block(&mut self) -> Option<*mut BlockInfo> {
        let call_mmap = |:| os::MemoryMap::new(BLOCK_SIZE,
            &[os::MapOption::MapReadable, os::MapOption::MapWritable]).ok();
        let wrap_mmap = |:mmap: os::MemoryMap| unsafe {
            let object = mmap.data() as *mut BlockInfo;
            ptr::write(object, BlockInfo::new(mmap));
            object
        };
        return self.free_blocks.pop().or_else(call_mmap).map(wrap_mmap);
    }

    pub fn return_block(&mut self, block: *mut BlockInfo) {
        debug!("Returned block {:p}", block);
        let block_info = unsafe { ptr::read(block) };
        if self.free_blocks.len() < 10 {
            self.free_blocks.push(block_info.into_memory_map());
        }
    }

}

#[test]
fn get_and_return_single_block() {
    let mut block_allocator = BlockAllocator::new();
    let block = block_allocator.get_block().unwrap();
    block_allocator.return_block(block);
}

#[test]
fn get_and_return_multiple_blocks() {
    let mut block_allocator = BlockAllocator::new();
    let mut blocks = Vec::new();
    for _ in range(0, 50u) {
        blocks.push(block_allocator.get_block().unwrap());
    }
    for block in blocks.into_iter() {
        block_allocator.return_block(block);
    }
}
