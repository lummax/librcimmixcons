// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use spaces::immix_space::block_info::BlockInfo;
use spaces::immix_space::allocator::BlockTuple;
use spaces::immix_space::allocator::Allocator;

use std::collections::RingBuf;

use constants::{BLOCK_SIZE, LINE_SIZE};

pub struct EvacAllocator {
    unavailable_blocks: RingBuf<*mut BlockInfo>,
    evac_headroom: RingBuf<*mut BlockInfo>,
    current_block: Option<BlockTuple>,
}

impl EvacAllocator {
    pub fn new() -> EvacAllocator {
        return EvacAllocator {
            unavailable_blocks: RingBuf::new(),
            evac_headroom: RingBuf::new(),
            current_block: None,
        };
    }

    pub fn extend_evac_headroom(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.evac_headroom.extend(blocks.into_iter());
    }

    pub fn evac_headroom(&self) -> usize {
        return self.evac_headroom.len();
    }
}

impl Allocator for EvacAllocator {
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
        debug!("Request new block in evacuation");
        return self.evac_headroom.pop_front()
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
