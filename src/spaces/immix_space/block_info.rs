// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::{HashSet, VecMap};
use std::num::Int;

use constants::{BLOCK_SIZE, LINE_SIZE, NUM_LINES_PER_BLOCK};
use gc_object::GCObjectRef;

pub struct BlockInfo {
    line_counter: VecMap<u8>,
    object_map: HashSet<GCObjectRef>,
    new_objects: HashSet<GCObjectRef>,
    allocated: bool,
    hole_count: u8,
    evacuation_candidate: bool,
}

impl BlockInfo {
    pub fn new() -> BlockInfo {
        let mut line_counter = VecMap::with_capacity(NUM_LINES_PER_BLOCK);
        for index in (0..NUM_LINES_PER_BLOCK) {
            line_counter.insert(index, 0);
        }
        return BlockInfo {
            line_counter: line_counter,
            object_map: HashSet::new(),
            new_objects: HashSet::new(),
            allocated: false,
            hole_count: 0,
            evacuation_candidate: false,
        };
    }

    pub fn set_allocated(&mut self) {
        self.allocated = true;
    }

    pub fn set_gc_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_in_block(object),
            "set_gc_object() on invalid block: {:p} (allocated={})",
            self, self.allocated);
        self.object_map.insert(object);
    }

    pub fn unset_gc_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_in_block(object),
            "unset_gc_object() on invalid block: {:p} (allocated={})",
            self, self.allocated);
        self.object_map.remove(&object);
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        if self.is_in_block(object) {
            return self.object_map.contains(&object);
        }
        return false;
    }

    pub fn get_object_map(&mut self) -> HashSet<GCObjectRef> {
        return self.object_map.clone();
    }

    pub fn clear_object_map(&mut self) {
        self.object_map.clear();
    }

    pub fn set_new_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_in_block(object),
            "set_new_object() on invalid block: {:p} (allocated={})",
            self, self.allocated);
        self.new_objects.insert(object);
    }

    pub fn get_new_objects(&mut self) -> HashSet<GCObjectRef> {
        return self.new_objects.clone();
    }

    pub fn remove_new_objects_from_map(&mut self) {
        let new_objects = self.new_objects.drain().collect();
        let difference = self.object_map.difference(&new_objects)
                                        .map(|o| *o).collect();
        self.object_map = difference;
    }

    pub fn set_evacuation_candidate(&mut self, hole_count: u8) {
        debug!("Set block {:p} to evacuation_candidate={} ({} holes)",
               &self, self.hole_count >= hole_count, self.hole_count);
        self.evacuation_candidate = self.hole_count >= hole_count;
    }

    pub fn is_evacuation_candidate(&self) -> bool{
        return self.evacuation_candidate;
    }

    pub fn increment_lines(&mut self, object: GCObjectRef) {
        self.update_line_nums(object, true);
    }

    pub fn decrement_lines(&mut self, object: GCObjectRef) {
        self.update_line_nums(object, false);
    }

    pub fn count_holes_and_marked_lines(&self) -> (u8, u8) {
        return (self.hole_count,
                self.line_counter.values().filter(|&e| *e != 0).count() as u8);
    }

    pub fn count_holes_and_available_lines(&self) -> (u8, u8) {
        return (self.hole_count,
                self.line_counter.values().filter(|&e| *e == 0).count() as u8);
    }

    pub fn clear_line_counts(&mut self) {
        for index in (0..NUM_LINES_PER_BLOCK) {
            self.line_counter.insert(index, 0);
        }
    }

    pub fn reset(&mut self) {
        self.clear_line_counts();
        self.clear_object_map();
        self.allocated = false;
        self.hole_count = 0;
        self.evacuation_candidate = false;
    }

    pub fn is_empty(&self) -> bool {
        return self.line_counter.values().all(|v| *v == 0);
    }

    pub fn offset(&mut self, offset: usize) -> GCObjectRef {
        let self_ptr = self as *mut BlockInfo;
        let object = unsafe { (self_ptr as *mut u8).offset(offset as isize) };
        return object as GCObjectRef;
    }

    pub fn scan_block(&self, last_high_offset: u16) -> Option<(u16, u16)> {
        let last_high_index = last_high_offset as usize / LINE_SIZE;
        debug!("Scanning block {:p} for a hole with last_high_offset {}",
               self, last_high_index);
        let mut low_index = NUM_LINES_PER_BLOCK - 1;
        for index in ((last_high_index + 1)..NUM_LINES_PER_BLOCK) {
            if self.line_counter.get(&index).map_or(true, |c| *c == 0) {
                // +1 to skip the next line in case an object straddles lines
                low_index = index + 1;
                break;
            }
        }
        let mut high_index = NUM_LINES_PER_BLOCK;
        for index in (low_index..NUM_LINES_PER_BLOCK) {
            if self.line_counter.get(&index).map_or(false, |c| *c != 0) {
                high_index = index;
                break;
            }
        }
        if low_index == high_index && high_index != (NUM_LINES_PER_BLOCK - 1) {
            debug!("Rescan: Found single line hole? in block {:p}", self);
            return self.scan_block((high_index * LINE_SIZE - 1) as u16);
        } else if low_index < (NUM_LINES_PER_BLOCK - 1) {
            debug!("Found low index {} and high index {} in block {:p}",
                   low_index, high_index, self);
            return Some(((low_index * LINE_SIZE) as u16,
                         (high_index * LINE_SIZE - 1) as u16));
        }
        debug!("Found no hole in block {:p}", self);
        return None;
    }

    pub fn count_holes(&mut self) {
        let holes = self.line_counter.values()
            .fold((0, false), |(holes, in_hole), &elem|
                  match (in_hole, elem) {
                    (false, 0) => (holes + 1, true),
                    (_, _) => (holes, false),
                  }).0;
        self.hole_count = holes;
    }
}

impl BlockInfo{
    fn is_in_block(&self, object: GCObjectRef) -> bool {
        // This works because we get zeroed memory from the OS, so
        // self.allocated will be false if this block is not initialized and
        // this method gets only called for objects within the ImmixSpace.
        // After the first initialization the field is properly managed.
        if self.allocated {
            let self_ptr = self as *const BlockInfo as *const u8;
            let self_bound = unsafe{ self_ptr.offset(BLOCK_SIZE as isize)};
            return self_ptr < (object as *const u8)
                && (object as *const u8) <  self_bound;
        }
        return false;
    }


    fn object_to_line_num(object: GCObjectRef) -> usize {
        return (object as usize % BLOCK_SIZE) / LINE_SIZE;
    }

    fn update_line_nums(&mut self, object: GCObjectRef, increment: bool) {
        // This calculates how many lines are affected starting from a
        // LINE_SIZE aligned address. So it might not mark enough lines. But
        // that does not matter as we always skip a line in scan_block()
        let line_num = BlockInfo::object_to_line_num(object);
        let object_size = unsafe{ (*object).object_size() };
        for line in line_num..(line_num + (object_size / LINE_SIZE) + 1) {
            match increment {
                true => {
                    if self.line_counter.contains_key(&line) {
                        if let Some(val) = self.line_counter.get_mut(&line) {
                            *val += 1;
                        }
                    } else { self.line_counter.insert(line, 1); }
                    debug!("Incremented line count for line {} to {}", line,
                           self.line_counter.get(&line).unwrap());
                },
                false => {
                    if self.line_counter.contains_key(&line) {
                        if let Some(val) = self.line_counter.get_mut(&line) {
                            *val = Int::saturating_sub(*val, 1);
                        }
                    } else { self.line_counter.insert(line, 0); }
                    debug!("Decremented line count for line {} to {}", line,
                           self.line_counter.get(&line).unwrap());
                }
            }
        }
    }
}
