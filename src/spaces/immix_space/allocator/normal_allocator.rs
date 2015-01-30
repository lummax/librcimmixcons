// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use spaces::immix_space::block_info::BlockInfo;
use spaces::immix_space::block_allocator::BlockAllocator;
use spaces::immix_space::allocator::BlockTuple;
use spaces::immix_space::allocator::Allocator;

use std::collections::RingBuf;
use std::rc::Rc;
use std::cell::RefCell;

use constants::{BLOCK_SIZE, LINE_SIZE};

pub struct NormalAllocator {
    block_allocator: Rc<RefCell<BlockAllocator>>,
    unavailable_blocks: RingBuf<*mut BlockInfo>,
    recyclable_blocks: RingBuf<*mut BlockInfo>,
    current_block: Option<BlockTuple>,
}

impl NormalAllocator {
    pub fn new(block_allocator: Rc<RefCell<BlockAllocator>>) -> NormalAllocator {
        return NormalAllocator {
            block_allocator: block_allocator,
            unavailable_blocks: RingBuf::new(),
            recyclable_blocks: RingBuf::new(),
            current_block: None,
        };
    }

    pub fn set_recyclable_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.recyclable_blocks = blocks;
    }

    pub fn get_all_blocks(&mut self) -> RingBuf<*mut BlockInfo> {
        return self.unavailable_blocks.drain()
                   .chain(self.recyclable_blocks.drain())
                   .chain(self.current_block.take().map(|b| b.0).into_iter())
                   .collect();
    }

}

impl Allocator for NormalAllocator {
    fn take_current_block(&mut self) -> Option<BlockTuple> {
        return self.current_block.take();
    }

    fn put_current_block(&mut self, block_tuple: BlockTuple) {
        self.current_block = Some(block_tuple);
    }

    fn get_new_block(&mut self) -> Option<BlockTuple> {
        debug!("Request new block");
        return self.block_allocator.borrow_mut()
                   .get_block()
                   .map(|b| unsafe{ (*b).set_allocated(); b })
                   .map(|block| (block, LINE_SIZE as u16, (BLOCK_SIZE - 1) as u16));
    }

    fn handle_no_hole(&mut self, size: usize) -> Option<BlockTuple> {
        if size >= LINE_SIZE {
            return None;
        }
        return match self.recyclable_blocks.pop_front() {
            None => None,
            Some(block) => match unsafe{ (*block).scan_block((LINE_SIZE - 1) as u16) } {
                None => {
                    self.handle_full_block(block);
                    self.handle_no_hole(size)
                },
                Some((low, high)) => self.scan_for_hole(size, (block, low, high))
                                         .or_else(|| self.handle_no_hole(size)),
            }
        };
    }

    fn handle_full_block(&mut self, block: *mut BlockInfo) {
        debug!("Push block {:p} into unavailable_blocks", block);
        self.unavailable_blocks.push_back(block);
    }
}
