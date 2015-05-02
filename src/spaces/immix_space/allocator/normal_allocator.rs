// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use spaces::immix_space::block_info::BlockInfo;
use spaces::immix_space::block_allocator::BlockAllocator;
use spaces::immix_space::allocator::BlockTuple;
use spaces::immix_space::allocator::Allocator;

use std::rc::Rc;
use std::cell::RefCell;

use constants::{BLOCK_SIZE, LINE_SIZE};

/// The `NormalAllocator` is the standard allocator to allocate objects within
/// the immix space.
///
/// Objects smaller than `MEDIUM_OBJECT` bytes are
pub struct NormalAllocator {
    /// The global `BlockAllocator` to get new blocks from.
    block_allocator: Rc<RefCell<BlockAllocator>>,

    /// The exhausted blocks.
    unavailable_blocks: Vec<*mut BlockInfo>,

    /// The blocks with holes to recycle before requesting new blocks..
    recyclable_blocks: Vec<*mut BlockInfo>,

    /// The current block to allocate from.
    current_block: Option<BlockTuple>,
}

impl NormalAllocator {
    /// Create a new `NormalAllocator` backed by the given `BlockAllocator`.
    pub fn new(block_allocator: Rc<RefCell<BlockAllocator>>) -> NormalAllocator {
        return NormalAllocator {
            block_allocator: block_allocator,
            unavailable_blocks: Vec::new(),
            recyclable_blocks: Vec::new(),
            current_block: None,
        };
    }

    /// Set the recyclable blocks.
    pub fn set_recyclable_blocks(&mut self, blocks: Vec<*mut BlockInfo>) {
        self.recyclable_blocks = blocks;
    }
}

impl Allocator for NormalAllocator {
    fn get_all_blocks(&mut self) -> Vec<*mut BlockInfo> {
        return self.unavailable_blocks.drain(..)
                   .chain(self.recyclable_blocks.drain(..))
                   .chain(self.current_block.take().map(|b| b.0))
                   .collect();
    }

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
        return match self.recyclable_blocks.pop() {
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
        self.unavailable_blocks.push(block);
    }
}
