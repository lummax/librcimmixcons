// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::VecMap;
use std::num::Int;
use std::{os, mem};

use gc_object::GCObjectRef;
use constants::{BLOCK_SIZE, LINE_SIZE, NUM_LINES_PER_BLOCK};

pub struct BlockInfo {
    mmap: os::MemoryMap,
    line_counter: VecMap<u8>,
}

impl BlockInfo {
    pub fn new(mmap: os::MemoryMap) -> BlockInfo {
        debug_assert!(mmap.len() > mem::size_of::<BlockInfo>());
        let mut block = BlockInfo {
            mmap: mmap,
            line_counter: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
        };
        block.clear_line_counts();
        return block;
    }

    pub fn into_memory_map(self) -> os::MemoryMap {
        return self.mmap;
    }

    pub fn increment_lines(&mut self, object: GCObjectRef) {
        self.update_line_nums(object, true);
    }

    pub fn decrement_lines(&mut self, object: GCObjectRef) {
        self.update_line_nums(object, false);
    }

    pub fn clear_line_counts(&mut self) {
        for index in range(0, NUM_LINES_PER_BLOCK) {
            self.line_counter.insert(index, 0);
        }
    }

    pub fn is_empty(&self) -> bool {
        return self.line_counter.values().all(|v| *v == 0);
    }

    pub fn offset(&self, offset: uint) -> GCObjectRef {
        return unsafe{ self.mmap.data().offset(offset as int) } as GCObjectRef;
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

    pub fn count_holes_and_marked_lines(&self) -> (u8, u8) {
        let (holes, _, marked_lines) = self.line_counter.values()
            .fold((0, false, 0), |(holes, in_hole, marked_lines), &elem|
                  match (in_hole, elem) {
                    (true, 0) => (holes, true, marked_lines),
                    (true, _) => (holes, false, marked_lines + 1),
                    (false, 0) => (holes + 1, true, marked_lines),
                    (false, _) => (holes, false, marked_lines + 1),
                  });
        return (holes, marked_lines);
    }
}

impl BlockInfo {
    fn object_to_line_num(object: GCObjectRef) -> uint {
        return (object as uint % BLOCK_SIZE) / LINE_SIZE;
    }

    fn update_line_nums(&mut self, object: GCObjectRef, increment: bool) {
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
}

