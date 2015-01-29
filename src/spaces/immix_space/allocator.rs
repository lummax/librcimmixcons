// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use spaces::immix_space::block_info::BlockInfo;
use spaces::immix_space::block_allocator::BlockAllocator;

use std::collections::RingBuf;

use constants::{BLOCK_SIZE, LINE_SIZE};
use gc_object::GCObjectRef;

type BlockTuple = (*mut BlockInfo, u16, u16);

pub struct Allocator {
    block_allocator: BlockAllocator,
    unavailable_blocks: RingBuf<*mut BlockInfo>,
    recyclable_blocks: RingBuf<*mut BlockInfo>,
    evac_headroom: RingBuf<*mut BlockInfo>,
    current_block: Option<BlockTuple>,
    overflow_block: Option<BlockTuple>,
}

impl Allocator {
    pub fn new() -> Allocator {
        return Allocator {
            block_allocator: BlockAllocator::new(),
            unavailable_blocks: RingBuf::new(),
            recyclable_blocks: RingBuf::new(),
            evac_headroom: RingBuf::new(),
            current_block: None,
            overflow_block: None,
        };
    }

    pub fn allocate(&mut self, size: usize, perform_evac: bool) -> Option<GCObjectRef> {
        return if size < LINE_SIZE {
            self.current_block.take()
                              .and_then(|tp| self.scan_for_hole(size, tp))
        } else {
            self.overflow_block.take()
                               .and_then(|tp| self.scan_for_hole(size, tp))
                               .or_else(|| self.get_new_block(perform_evac))
        }.or_else(|| self.scan_recyclables(size))
         .or_else(|| self.get_new_block(perform_evac))
         .map(|tp| self.allocate_from_block(size, tp))
         .map(|(tp, object)| {
             if size < LINE_SIZE { self.current_block = Some(tp);
             } else { self.overflow_block = Some(tp); }
             valgrind_malloclike!(object, size);
             object
         });
    }

    pub fn is_in_space(&self, object: GCObjectRef) -> bool {
        return self.block_allocator.is_in_space(object);
    }

    pub fn set_unavailable_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.unavailable_blocks = blocks;
    }

    pub fn set_recyclable_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.recyclable_blocks = blocks;
    }

    pub fn extend_evac_headroom(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.evac_headroom.extend(blocks.into_iter());
    }

    pub fn evac_headroom(&self) -> usize {
        return self.evac_headroom.len();
    }

    pub fn block_allocator(&mut self) -> &mut BlockAllocator {
        return &mut self.block_allocator;
    }

    pub fn get_all_blocks(&mut self) -> RingBuf<*mut BlockInfo> {
        return self.unavailable_blocks.drain()
                   .chain(self.recyclable_blocks.drain())
                   .chain(self.current_block.take().map(|b| b.0).into_iter())
                   .collect();
    }

}

impl Allocator {
    fn scan_for_hole(&mut self, size: usize, block_tuple: BlockTuple)
        -> Option<BlockTuple> {
            let (block, low, high) = block_tuple;
            return match (high - low) as usize >= size {
                true => {
                    debug!("Found hole in block {:p}", block);
                    Some(block_tuple)
                },
                false => match unsafe{ (*block).scan_block(high) } {
                    None => {
                        debug!("Push block {:p} into unavailable_blocks", block);
                        self.unavailable_blocks.push_back(block);
                        None
                    },
                    Some((low, high)) =>
                        self.scan_for_hole(size, (block, low, high)),
                }
            };
        }

    fn scan_recyclables(&mut self, size: usize) -> Option<BlockTuple> {
        return match self.recyclable_blocks.pop_front() {
            None => None,
            Some(block) => match unsafe{ (*block).scan_block((LINE_SIZE - 1) as u16) } {
                None => {
                    debug!("Push block {:p} into unavailable_blocks", block);
                    self.unavailable_blocks.push_back(block);
                    self.scan_recyclables(size)
                },
                Some((low, high)) => self.scan_for_hole(size, (block, low, high))
                                         .or_else(|| self.scan_recyclables(size)),
            }
        };
    }

    fn allocate_from_block(&mut self, size: usize, block_tuple: BlockTuple)
        -> (BlockTuple, GCObjectRef) {
            let (block, low, high) = block_tuple;
            let object = unsafe { (*block).offset(low as usize) };
            debug!("Allocated object {:p} of size {} in {:p} (object={})",
                   object, size, block, size >= LINE_SIZE);
            return ((block, low + size as u16, high), object);
        }

    fn get_new_block(&mut self, perform_evac: bool) -> Option<BlockTuple> {
        return if perform_evac {
            debug!("Request new block in evacuation");
            self.evac_headroom.pop_front()
        } else {
            debug!("Request new block");
            self.block_allocator.get_block()
        }.map(|b| unsafe{ (*b).set_allocated(); b })
         .map(|block| (block, LINE_SIZE as u16, (BLOCK_SIZE - 1) as u16));
    }
}
