// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

use std::collections::{RingBuf, HashSet, VecMap};
use std::num::Int;
use std::{mem, ptr, os};

use constants::{BLOCK_SIZE, LINE_SIZE,
                NUM_LINES_PER_BLOCK, HEAP_SIZE, EVAC_HEADROOM,
                CICLE_TRIGGER_THRESHHOLD, EVAC_TRIGGER_THRESHHOLD};
use gc_object::{GCRTTI, GCObject, GCObjectRef};

struct BlockInfo {
    line_counter: VecMap<u8>,
    hole_count: u8,
    evacuation_candidate: bool,
}

impl BlockInfo {
    fn new() -> BlockInfo {
        let mut line_counter = VecMap::with_capacity(NUM_LINES_PER_BLOCK);
        for index in (0..NUM_LINES_PER_BLOCK) {
            line_counter.insert(index, 0);
        }
        return BlockInfo {
            line_counter: line_counter,
            hole_count: 0,
            evacuation_candidate: false,
        };
    }

    fn set_evacuation_candidate(&mut self, hole_count: u8) {
        debug!("Set block {:p} to evacuation_candidate={} ({} holes)",
               &self, self.hole_count >= hole_count, self.hole_count);
        self.evacuation_candidate = self.hole_count >= hole_count;
    }

    fn is_evacuation_candidate(&self) -> bool{
        return self.evacuation_candidate;
    }

    fn increment_lines(&mut self, object: GCObjectRef) {
        self.update_line_nums(object, true);
    }

    fn decrement_lines(&mut self, object: GCObjectRef) {
        self.update_line_nums(object, false);
    }

    fn count_holes_and_marked_lines(&self) -> (u8, u8) {
        return (self.hole_count,
                self.line_counter.values().filter(|&e| *e != 0).count() as u8);
    }

    fn count_holes_and_available_lines(&self) -> (u8, u8) {
        return (self.hole_count,
                self.line_counter.values().filter(|&e| *e == 0).count() as u8);
    }

    fn clear_line_counts(&mut self) {
        for index in (0..NUM_LINES_PER_BLOCK) {
            self.line_counter.insert(index, 0);
        }
    }

    fn reset(&mut self) {
        self.clear_line_counts();
        self.hole_count = 0;
        self.evacuation_candidate = false;
    }

    fn is_empty(&self) -> bool {
        return self.line_counter.values().all(|v| *v == 0);
    }

    fn offset(&mut self, offset: usize) -> GCObjectRef {
        unsafe {
            let self_ptr = self as *mut BlockInfo;
            let object = (self_ptr as *mut u8).offset(offset as isize);
            return object as GCObjectRef;
        }
    }

    fn scan_block(&self, last_high_offset: u16) -> Option<(u16, u16)> {
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

    fn count_holes(&mut self) {
        let holes = self.line_counter.values()
            .fold((0, false), |(holes, in_hole), &elem|
                  match (in_hole, elem) {
                    (false, 0) => (holes + 1, true),
                    (_, _) => (holes, false),
                  }).0;
        self.hole_count = holes;
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

struct BlockAllocator {
    mmap: os::MemoryMap,
    data: *mut u8,
    data_bound: *mut u8,
    free_blocks: Vec<*mut BlockInfo>,
}

impl BlockAllocator {
    fn new() -> BlockAllocator {
        let mmap = os::MemoryMap::new(HEAP_SIZE + BLOCK_SIZE,
                                      &[os::MapOption::MapReadable,
                                        os::MapOption::MapWritable]).unwrap();
        let data = unsafe{ mmap.data().offset(((mmap.data() as usize) % BLOCK_SIZE) as isize) };
        let data_bound = unsafe{ mmap.data().offset(mmap.len() as isize) };
        debug!("Allocated heap of size {}, usable range: {:p} - {:p} (size {}, {} blocks)",
                HEAP_SIZE + BLOCK_SIZE, data, data_bound,
                (data_bound as usize) - (data  as usize),
                ((data_bound as usize) - (data  as usize)) / BLOCK_SIZE);
        return BlockAllocator {
            mmap: mmap,
            data: data,
            data_bound: data_bound,
            free_blocks: Vec::with_capacity(HEAP_SIZE / BLOCK_SIZE),
        };
    }

    fn build_next_block(&mut self) -> Option<*mut BlockInfo> {
        let block = unsafe{ self.data.offset(BLOCK_SIZE as isize) };
        if block < self.data_bound {
            self.data = block;
            unsafe{ ptr::write(block as *mut BlockInfo, BlockInfo::new()); }
            return Some(block as *mut BlockInfo);
        }
        return None;
    }

    fn get_block(&mut self) -> Option<*mut BlockInfo> {
        return self.free_blocks.pop()
                               .map(|b| { unsafe{ (*b).reset() }; b } )
                               .or_else(|| self.build_next_block());
    }

    fn return_block(&mut self, block: *mut BlockInfo) {
        debug!("Returned block {:p}", block);
        self.free_blocks.push(block);
    }

    fn total_blocks(&self) -> usize {
        return HEAP_SIZE / BLOCK_SIZE;
    }

    fn available_blocks(&self) -> usize {
        return (((self.data_bound as usize) - (self.data as usize)) % BLOCK_SIZE)
            + self.free_blocks.len();
    }
}

type BlockTuple = (*mut BlockInfo, u16, u16);

pub struct ImmixSpace {
    block_allocator: BlockAllocator,
    object_map: HashSet<GCObjectRef>,
    object_map_backup: HashSet<GCObjectRef>,
    mark_histogram: VecMap<u8>,
    unavailable_blocks: RingBuf<*mut BlockInfo>,
    recyclable_blocks: RingBuf<*mut BlockInfo>,
    evac_headroom: RingBuf<*mut BlockInfo>,
    current_block: Option<BlockTuple>,
    overflow_block: Option<BlockTuple>,
    current_live_mark: bool,
    perform_evac: bool,
}

impl ImmixSpace {
    pub fn new() -> ImmixSpace {
        return ImmixSpace {
            block_allocator: BlockAllocator::new(),
            object_map: HashSet::new(),
            object_map_backup: HashSet::new(),
            mark_histogram: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
            unavailable_blocks: RingBuf::new(),
            recyclable_blocks: RingBuf::new(),
            evac_headroom: RingBuf::new(),
            current_block: None,
            overflow_block: None,
            current_live_mark: false,
            perform_evac: false,
        };
    }

    pub fn set_gc_object(&mut self, object: GCObjectRef) {
        self.object_map.insert(object);
    }

    pub fn unset_gc_object(&mut self, object: GCObjectRef) {
        self.object_map.remove(&object);
    }

    pub fn is_gc_object(&self, object: GCObjectRef) -> bool {
        return self.object_map.contains(&object);
    }

    pub fn current_live_mark(&self) -> bool {
        return self.current_live_mark;
    }

    pub fn decrement_lines(&mut self, object: GCObjectRef) {
        unsafe{ (*self.get_block_ptr(object)).decrement_lines(object); }
    }

    pub fn increment_lines(&mut self, object: GCObjectRef) {
        unsafe{ (*self.get_block_ptr(object)).increment_lines(object); }
    }

    pub fn allocate(&mut self, rtti: *const GCRTTI) -> Option<GCObjectRef> {
        let size = unsafe{ (*rtti).object_size() };
        debug!("Request to allocate an object of size {}", size);
        if let Some(object) = self.raw_allocate(size) {
            unsafe { ptr::write(object, GCObject::new(rtti, self.current_live_mark)); }
            return Some(object);
        }
        return None;
    }

    pub fn maybe_evacuate(&mut self, object: GCObjectRef) -> Option<GCObjectRef> {
        let block_info = unsafe{ self.get_block_ptr(object) };
        let is_pinned = unsafe{ (*object).is_pinned() };
        let is_candidate = unsafe{ (*block_info).is_evacuation_candidate() };
        if is_pinned || !is_candidate {
            return None;
        }
        let size = unsafe{ (*object).object_size() };
        if let Some(new_object) = self.raw_allocate(size) {
            unsafe{
                ptr::copy_nonoverlapping_memory(new_object as *mut u8,
                                                object as *const u8, size);
                debug_assert!(*object == *new_object,
                              "Evacuated object was not copied correcty");
                (*object).set_forwarded(new_object);
                self.unset_gc_object(object);
            }
            debug!("Evacuated object {:p} from block {:p} to {:p}", object,
                   block_info, new_object);
            valgrind_freelike!(object);
            return Some(new_object);
        }
        debug!("Can't evacuation object {:p} from block {:p}", object, block_info);
        return None;
    }

    pub fn prepare_collection(&mut self, evacuation: bool, cycle_collect: bool) -> bool {
        self.unavailable_blocks.extend(self.recyclable_blocks.drain());
        self.unavailable_blocks.extend(self.current_block.take()
                                           .map(|b| b.0).into_iter());

        let available_blocks = self.block_allocator.available_blocks();
        let total_blocks = self.block_allocator.total_blocks();
        if !evacuation {
            let evac_threshhold = ((total_blocks as f32) * EVAC_TRIGGER_THRESHHOLD) as usize;
            let available_evac_blocks = available_blocks + self.evac_headroom.len();
            if available_evac_blocks < evac_threshhold {
                let hole_threshhold = self.establish_hole_threshhold();
                self.perform_evac = hole_threshhold > 0
                    && hole_threshhold < NUM_LINES_PER_BLOCK as u8;
                if self.perform_evac {
                    debug!("Performing evacuation with hole_threshhold={} and evac_headroom={}",
                           hole_threshhold, self.evac_headroom.len());
                    for block in self.unavailable_blocks.iter_mut() {
                        unsafe{ (**block).set_evacuation_candidate(hole_threshhold); }
                    }
                }
            }
        } else { self.perform_evac = true; }

        if !cycle_collect {
            let cycle_theshold = ((total_blocks as f32) * CICLE_TRIGGER_THRESHHOLD) as usize;
            return self.block_allocator.available_blocks() < cycle_theshold;
        }
        return true;
    }

    pub fn complete_collection(&mut self) {
        self.mark_histogram.clear();
        self.perform_evac = false;
        self.sweep_unavailable_blocks();
    }

    pub fn prepare_immix_collection(&mut self) {
        for block in self.unavailable_blocks.iter_mut() {
            unsafe{ (**block).clear_line_counts(); }
        }

        if cfg!(feature = "valgrind") {
            self.object_map_backup = self.object_map.clone();
        }
        self.object_map.clear();
    }

    pub fn complete_immix_collection(&mut self) {
        self.current_live_mark = !self.current_live_mark;
        if cfg!(feature = "valgrind") {
            for &object in self.object_map_backup.difference(&self.object_map) {
                valgrind_freelike!(object);
            }
            self.object_map_backup.clear();
        }
    }
}

impl ImmixSpace {
    unsafe fn get_block_ptr(&mut self, object: GCObjectRef) -> *mut BlockInfo {
        let block_offset = object as usize % BLOCK_SIZE;
        return mem::transmute((object as *mut u8).offset(-(block_offset as isize)));
    }

    fn raw_allocate(&mut self, size: usize) -> Option<GCObjectRef> {
        return if size < LINE_SIZE {
            self.current_block.take()
                              .and_then(|tp| self.scan_for_hole(size, tp))
        } else {
            self.overflow_block.take()
                               .and_then(|tp| self.scan_for_hole(size, tp))
                               .or_else(|| self.get_new_block())
        }.or_else(|| self.scan_recyclables(size))
         .or_else(|| self.get_new_block())
         .map(|tp| self.allocate_from_block(size, tp))
         .map(|(tp, object)| {
             if size < LINE_SIZE { self.current_block = Some(tp);
             } else { self.overflow_block = Some(tp); }
             valgrind_malloclike!(object, size);
             self.set_gc_object(object);
             object
         });
    }

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

    fn get_new_block(&mut self) -> Option<BlockTuple> {
        return if self.perform_evac {
            debug!("Request new block in evacuation");
            self.evac_headroom.pop_front()
        } else {
            debug!("Request new block");
            self.block_allocator.get_block()
        }.map(|block| (block, LINE_SIZE as u16, (BLOCK_SIZE - 1) as u16));
    }

    fn sweep_unavailable_blocks(&mut self) {
        let mut unavailable_blocks = RingBuf::new();
        for block in self.unavailable_blocks.drain() {
            if unsafe{ (*block).is_empty() } {
                // XXX We should not use a constant here, but something that
                // XXX changes dynamically (see rcimmix: MAX heuristic).
                if self.evac_headroom.len() < EVAC_HEADROOM {
                    debug!("Buffer free block {:p} for evacuation", block);
                    unsafe{ (*block).reset() ;}
                    self.evac_headroom.push_back(block);
                } else {
                    debug!("Return block {:p} to global block allocator", block);
                    self.block_allocator.return_block(block);
                }
            } else {
                unsafe{ (*block).count_holes(); }
                let (holes, marked_lines) = unsafe{ (*block).count_holes_and_marked_lines() };
                if self.mark_histogram.contains_key(&(holes as usize)) {
                    if let Some(val) = self.mark_histogram.get_mut(&(holes as usize)) {
                        *val += marked_lines;
                    }
                } else { self.mark_histogram.insert(holes as usize, marked_lines); }
                debug!("Found {} holes and {} marked lines in block {:p}",
                       holes, marked_lines, block);
                match holes {
                    0 => {
                        debug!("Push block {:p} into unavailable_blocks", block);
                        unavailable_blocks.push_back(block);
                    },
                    _ => {
                        debug!("Push block {:p} into recyclable_blocks", block);
                        self.recyclable_blocks.push_back(block);
                    }
                }
            }
        }
        self.unavailable_blocks.extend(unavailable_blocks.into_iter());
    }

    fn establish_hole_threshhold(&self) -> u8 {
        let mut available_histogram : VecMap<u8> = VecMap::with_capacity(NUM_LINES_PER_BLOCK);
        for block in self.unavailable_blocks.iter() {
            let (holes, free_lines) = unsafe{ (**block).count_holes_and_available_lines() };
            if available_histogram.contains_key(&(holes as usize)) {
                if let Some(val) = available_histogram.get_mut(&(holes as usize)) {
                    *val += free_lines;
                }
            } else { available_histogram.insert(holes as usize, free_lines); }
        }
        let mut required_lines = 0 as u8;
        let mut available_lines = (self.evac_headroom.len() * (NUM_LINES_PER_BLOCK - 1)) as u8;

        for threshold in (0..NUM_LINES_PER_BLOCK) {
            required_lines += *self.mark_histogram.get(&threshold).unwrap_or(&0);
            available_lines -= *available_histogram.get(&threshold).unwrap_or(&0);
            if available_lines <= required_lines {
                return threshold as u8;
            }
        }
        return NUM_LINES_PER_BLOCK as u8;
    }
}
