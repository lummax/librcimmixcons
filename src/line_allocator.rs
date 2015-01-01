use std::collections::{RingBuf, HashSet};
use std::mem;
use std::ptr;

use block_allocator::BlockAllocator;
use block_info::BlockInfo;
use constants::{BLOCK_SIZE, LINE_SIZE};
use gc_object::GCObject;

pub struct LineAllocator {
    block_allocator: BlockAllocator,
    object_map: HashSet<*mut GCObject>,
    unavailable_blocks: RingBuf<*mut BlockInfo>,
    recyclable_blocks: RingBuf<*mut BlockInfo>,
    current_block: Option<(*mut BlockInfo, u16, u16)>,
}

impl LineAllocator {
    pub fn new(block_allocator: BlockAllocator) -> LineAllocator {
        return LineAllocator {
            block_allocator: block_allocator,
            object_map: HashSet::new(),
            unavailable_blocks: RingBuf::new(),
            recyclable_blocks: RingBuf::new(),
            current_block: None,
        };
    }

    pub fn is_gc_object(&self, object: *mut GCObject) -> bool {
        return self.object_map.contains(&object);
    }

    pub fn allocate(&mut self, size: uint, variables: uint) -> Option<*mut GCObject> {
        debug!("Request to allocate an object of size {}", size);
        let block_tuple = self.current_block
                              .and_then(|tp| self.scan_for_hole(size, tp))
                              .or_else(|| self.scan_recyclables(size))
                              .or_else(|| self.get_new_block());
        return match block_tuple {
            None => None,
            Some((block, low, high)) => {
                self.current_block = Some((block, low + size as u16, high));
                let object = unsafe { (*block).offset(low as uint) };
                self.object_map.insert(object);
                unsafe { ptr::write(object, GCObject::new(size, variables)); }
                debug!("Allocated object {} of size {} in {}", object, size, block);
                Some(object)
            }
        };
    }

    fn scan_for_hole(&mut self, size: uint, tuple: (*mut BlockInfo, u16, u16))
        -> Option<(*mut BlockInfo, u16, u16)> {
            let (block, low, high) = tuple;
            return match (high - low) as uint >= size {
                true => {
                    debug!("Found hole in block {:p}", block);
                    Some(tuple)
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

    fn scan_recyclables(&mut self, size: uint) -> Option<(*mut BlockInfo, u16, u16)> {
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

    fn get_new_block(&mut self) -> Option<(*mut BlockInfo, u16, u16)> {
        debug!("Request new block from block_allocator");
        return self.block_allocator.get_block()
                   .map(|block| (block, LINE_SIZE as u16, (BLOCK_SIZE - 1) as u16));
    }

    pub fn return_empty_blocks(&mut self) {
        if let Some((block, _, _)) = self.current_block.take() {
            self.unavailable_blocks.push_back(block);
        }
        for block in self.unavailable_blocks.drain() {
            if unsafe{ (*block).is_empty() } {
                debug!("Return block {:p} to global block allocator", block);
                self.block_allocator.return_block(block);
            } else {
                debug!("Recycle block {:p}", block);
                self.recyclable_blocks.push_back(block);
            }
        }
    }

    unsafe fn get_block_ptr(&mut self, object: *mut GCObject) -> *mut BlockInfo {
        let block_offset = object as uint % BLOCK_SIZE;
        return mem::transmute((object as *mut u8).offset(-(block_offset as int)));
    }

    pub fn decrement_lines(&mut self, object: *mut GCObject) {
        unsafe{
            let block_ptr = self.get_block_ptr(object);
            (*block_ptr).decrement_lines(object);
        }
    }

    pub fn increment_lines(&mut self, object: *mut GCObject) {
        unsafe{
            let block_ptr = self.get_block_ptr(object);
            (*block_ptr).increment_lines(object);
        }
    }
}
