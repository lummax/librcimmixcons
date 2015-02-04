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

/// The `OverflowAllocator` is used to allocate *medium* sized objects
/// (objects of at least `MEDIUM_OBJECT` bytes size) within the immix space to
/// limit fragmentation in the `NormalAllocator`.
pub struct OverflowAllocator {
    /// The global `BlockAllocator` to get new blocks from.
    block_allocator: Rc<RefCell<BlockAllocator>>,

    /// The exhausted blocks.
    unavailable_blocks: RingBuf<*mut BlockInfo>,

    /// The current block to allocate from.
    current_block: Option<BlockTuple>,
}

impl OverflowAllocator {
    /// Create a new `OverflowAllocator` backed by the given `BlockAllocator`.
    pub fn new(block_allocator: Rc<RefCell<BlockAllocator>>) -> OverflowAllocator {
        return OverflowAllocator {
            block_allocator: block_allocator,
            unavailable_blocks: RingBuf::new(),
            current_block: None,
        };
    }
}

impl Allocator for OverflowAllocator {
    fn get_all_blocks(&mut self) -> RingBuf<*mut BlockInfo> {
        return self.unavailable_blocks.drain()
                   .chain(self.current_block.take().map(|b| b.0).into_iter())
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

    #[allow(unused_variables)]
    fn handle_no_hole(&mut self, size: usize) -> Option<BlockTuple> {
        return None;
    }

    fn handle_full_block(&mut self, block: *mut BlockInfo) {
        debug!("Push block {:p} into unavailable_blocks", block);
        self.unavailable_blocks.push_back(block);
    }
}
