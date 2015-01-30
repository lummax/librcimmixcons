// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod normal_allocator;
mod overflow_allocator;
mod evac_allocator;

pub use self::normal_allocator::NormalAllocator;
pub use self::overflow_allocator::OverflowAllocator;
pub use self::evac_allocator::EvacAllocator;
use spaces::immix_space::block_info::BlockInfo;

use std::collections::RingBuf;

use constants::LINE_SIZE;
use gc_object::GCObjectRef;

type BlockTuple = (*mut BlockInfo, u16, u16);

pub trait Allocator {
    fn get_all_blocks(&mut self) -> RingBuf<*mut BlockInfo>;

    fn take_current_block(&mut self) -> Option<BlockTuple>;
    fn put_current_block(&mut self, block_tuple: BlockTuple);

    fn get_new_block(&mut self) -> Option<BlockTuple>;
    fn handle_no_hole(&mut self, size: usize) -> Option<BlockTuple>;
    fn handle_full_block(&mut self, block: *mut BlockInfo);

    fn allocate(&mut self, size: usize) -> Option<GCObjectRef> {
        return self.take_current_block()
                   .and_then(|tp| self.scan_for_hole(size, tp))
                   .or_else(|| self.handle_no_hole(size))
                   .or_else(|| self.get_new_block())
                   .map(|tp| self.allocate_from_block(size, tp))
                   .map(|(tp, object)| {
                       self.put_current_block(tp);
                       valgrind_malloclike!(object, size);
                       object
                   });
    }

    fn scan_for_hole(&mut self, size: usize, block_tuple: BlockTuple) -> Option<BlockTuple> {
        let (block, low, high) = block_tuple;
        return match (high - low) as usize >= size {
            true => {
                debug!("Found hole in block {:p}", block);
                Some(block_tuple)
            },
            false => match unsafe{ (*block).scan_block(high) } {
                None => {
                    self.handle_full_block(block);
                    None
                },
                Some((low, high)) => self.scan_for_hole(size, (block, low, high)),
            }
        };
    }

    fn allocate_from_block(&self, size: usize, block_tuple: BlockTuple)
        -> (BlockTuple, GCObjectRef) {
            let (block, low, high) = block_tuple;
            let object = unsafe { (*block).offset(low as usize) };
            debug!("Allocated object {:p} of size {} in {:p} (object={})",
                   object, size, block, size >= LINE_SIZE);
            return ((block, low + size as u16, high), object);
        }
}
