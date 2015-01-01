use std::collections::VecMap;
use std::num::Int;
use std::{os, mem};

use gc_object::GCObject;
use constants::{BLOCK_SIZE, LINE_SIZE, NUM_LINES_PER_BLOCK};

pub struct BlockInfo {
    mmap: os::MemoryMap,
    line_counter: VecMap<u8>,
}

impl BlockInfo {
    pub fn new(mmap: os::MemoryMap) -> BlockInfo {
        debug_assert!(mmap.len() > mem::size_of::<BlockInfo>());
        return BlockInfo {
            mmap: mmap,
            line_counter: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
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

    fn update_line_nums(&mut self, object: *const GCObject, increment: bool) {
        // This calculates how many lines are affected starting from a
        // LINE_SIZE aligned address. So it might not mark enough lines. But
        // that does not matter as we always skip a line in scan_block()
        let line_num = BlockInfo::object_to_line_num(object);
        let object_size = unsafe{ (*object).object_size() };
        for line in range(line_num, line_num + (object_size / LINE_SIZE) + 1) {
            match increment {
                true => {
                    self.line_counter.update(line, 1, |o, n| o + n);
                    debug!("Incremented line count for line {} to {}", line,
                           self.line_counter.get(&line).unwrap());
                },
                false => {
                    self.line_counter.update(line, 0,
                                             |o, _| Int::saturating_sub(o, 1));
                    debug!("Decremented line count for line {} to {}", line,
                           self.line_counter.get(&line).unwrap());
                }
            }
        }
    }

    pub fn increment_lines(&mut self, object: *const GCObject) {
        self.update_line_nums(object, true);
    }

    pub fn decrement_lines(&mut self, object: *const GCObject) {
        self.update_line_nums(object, false);
    }

    pub fn clear_line_counts(&mut self) {
        self.line_counter.clear();
    }

    pub fn is_empty(&self) -> bool {
        return self.line_counter.values().all(|v| *v == 0);
    }
}

