use std::collections::{Bitv, VecMap};
use std::num::Int;
use std::{os, mem};

use gc_object::GCObject;
use constants::{BLOCK_SIZE, LINE_SIZE, NUM_LINES_PER_BLOCK};

pub struct BlockInfo {
    mmap: os::MemoryMap,
    line_counter: VecMap<u8>,
    line_map: Bitv,
}

impl BlockInfo {
    pub fn new(mmap: os::MemoryMap) -> BlockInfo {
        debug_assert!(mmap.len() > mem::size_of::<BlockInfo>());
        return BlockInfo {
            mmap: mmap,
            line_counter: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
            line_map: Bitv::from_elem(NUM_LINES_PER_BLOCK, false),
        }
    }

    pub fn into_memory_map(self) -> os::MemoryMap {
        return self.mmap;
    }

    pub fn scan_block(&self, last_high_offset: u16) -> Option<(u16, u16)> {
        debug!("Scanning block {:p} for a hole", self);
        let last_high_index = last_high_offset as uint / LINE_SIZE;
        let mut low_index = NUM_LINES_PER_BLOCK - 1;
        for index in range(last_high_index + 1, NUM_LINES_PER_BLOCK) {
            if self.line_counter.get(&index).map_or(true, |c| *c == 0) {
                // +1 to skip the next line in case an object straddles lines
                low_index = index + 1;
                break;
            }
        }
        debug!("Found low index {} in block {:p}", low_index, self);
        let mut high_index = NUM_LINES_PER_BLOCK;
        for index in range(low_index + 1, NUM_LINES_PER_BLOCK) {
            if self.line_counter.get(&index).map_or(false, |c| *c != 0) {
                high_index = index;
                break;
            }
        }
        debug!("Found high index {} in block {:p}", high_index, self);
        return if low_index < (NUM_LINES_PER_BLOCK - 1) {
            Some(((low_index * LINE_SIZE) as u16,
            (high_index * LINE_SIZE - 1) as u16))
        } else {
            debug!("Found no hole in block {:p}", self);
            None
        };
    }

    pub fn offset(&mut self, offset: uint) -> *mut GCObject {
        return unsafe{ self.mmap.data().offset(offset as int) } as *mut GCObject;
    }

    fn object_to_line_num(object: *const GCObject) -> uint {
        return (object as uint % BLOCK_SIZE) / LINE_SIZE;
    }

    pub fn set_line_mark(&mut self, object: *const GCObject, mark: bool) {
        let line_num = BlockInfo::object_to_line_num(object);
        self.line_map.set(line_num, mark);
        debug!("Set line {} in block {:p}", mark, self);
    }

    pub fn get_line_mark(&self, object: *const GCObject) -> bool {
        return self.line_map
                   .get(BlockInfo::object_to_line_num(object))
                   .unwrap_or(false);
    }

    fn update_line_nums<F>(&mut self, object: *const GCObject, newval: u8,
                           f: F) where F: Fn(u8, u8) -> u8 {
        // This calculates how many lines are affected starting from a
        // LINE_SIZE aligned address. So it might not mark enough lines. But
        // that does not matter as we always skip a line in scan_block()
        let line_num = BlockInfo::object_to_line_num(object);
        let object_size = unsafe{ (*object).object_size() };
        for line in range(line_num, line_num + (object_size / LINE_SIZE) + 1) {
            self.line_counter.update(line, newval, |x, y| f(x, y));
            debug!("Change line count for line {} to {}", line,
                   self.line_counter.get(&line).unwrap());
        }
    }

    pub fn increment_lines(&mut self, object: *const GCObject) {
        debug!("Increment lines in block {:p}", self);
        self.update_line_nums(object, 1, |old, new| old + new);
    }

    pub fn decrement_lines(&mut self, object: *const GCObject) {
        debug!("Decrement lines in block {:p}", self);
        self.update_line_nums(object, 0, |old, _| Int::saturating_sub(old, 1));
    }

    pub fn is_empty(&self) -> bool {
        return self.line_counter.values().all(|v| *v == 0);
    }
}

