// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod rc_collector;
mod immix_collector;

pub use self::immix_collector::ImmixCollector;
pub use self::rc_collector::RCCollector;

use std::collections::{RingBuf, HashSet, VecMap};
use std::rc::Rc;
use std::cell::RefCell;

use constants::{NUM_LINES_PER_BLOCK, EVAC_HEADROOM,
                CICLE_TRIGGER_THRESHHOLD, EVAC_TRIGGER_THRESHHOLD};
use gc_object::GCObjectRef;
use spaces::immix_space::block_info::BlockInfo;
use spaces::immix_space::block_allocator::BlockAllocator;

pub struct Collector {
    block_allocator: Rc<RefCell<BlockAllocator>>,
    all_blocks: RingBuf<*mut BlockInfo>,
    object_map_backup: HashSet<GCObjectRef>,
    mark_histogram: VecMap<u8>,
}

impl Collector {
    pub fn new(block_allocator: Rc<RefCell<BlockAllocator>>) -> Collector {
        return Collector {
            block_allocator: block_allocator,
            all_blocks: RingBuf::new(),
            object_map_backup: HashSet::new(),
            mark_histogram: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
        };
    }

    pub fn extend_all_blocks(&mut self, blocks: RingBuf<*mut BlockInfo>) {
        self.all_blocks.extend(blocks.into_iter());
    }

    pub fn prepare_collection(&mut self, evacuation: bool, cycle_collect: bool,
                              evac_headroom: usize) -> (bool, bool) {
        let block_allocator = self.block_allocator.borrow();
        let available_blocks = block_allocator.available_blocks();
        let total_blocks = block_allocator.total_blocks();
        let mut perform_evac = evacuation;

        let evac_threshhold = ((total_blocks as f32) * EVAC_TRIGGER_THRESHHOLD) as usize;
        let available_evac_blocks = available_blocks + evac_headroom;
        if evacuation || available_evac_blocks < evac_threshhold {
            let hole_threshhold = self.establish_hole_threshhold(evac_headroom);
            perform_evac = hole_threshhold > 0
                && hole_threshhold < NUM_LINES_PER_BLOCK as u8;
            if perform_evac {
                debug!("Performing evacuation with hole_threshhold={} and evac_headroom={}",
                       hole_threshhold, evac_headroom);
                for block in self.all_blocks.iter_mut() {
                    unsafe{ (**block).set_evacuation_candidate(hole_threshhold); }
                }
            }
        }

        if !cycle_collect {
            let cycle_theshold = ((total_blocks as f32) * CICLE_TRIGGER_THRESHHOLD) as usize;
            return (block_allocator.available_blocks() < cycle_theshold, perform_evac);
        }
        return (true, perform_evac);
    }

    pub fn complete_collection(&mut self) -> (RingBuf<*mut BlockInfo>, RingBuf<*mut BlockInfo>){
        self.mark_histogram.clear();
        return self.sweep_all_blocks();
    }

    pub fn prepare_rc_collection(&mut self) {
        if cfg!(feature = "valgrind") {
            for block in self.all_blocks.iter_mut() {
                let block_new_objects = unsafe{ (**block).get_new_objects() };
                self.object_map_backup.extend(block_new_objects.into_iter());
            }
        }

        for block in self.all_blocks.iter_mut() {
            unsafe{ (**block).remove_new_objects_from_map(); }
        }
    }

    pub fn complete_rc_collection(&mut self) {
        if cfg!(feature = "valgrind") {
            let mut object_map = HashSet::new();
            for block in self.all_blocks.iter_mut() {
                let block_object_map = unsafe{ (**block).get_object_map() };
                object_map.extend(block_object_map.into_iter());
            }
            for &object in self.object_map_backup.difference(&object_map) {
                valgrind_freelike!(object);
            }
            self.object_map_backup.clear();
        }
    }

    pub fn prepare_immix_collection(&mut self) {
        if cfg!(feature = "valgrind") {
            for block in self.all_blocks.iter_mut() {
                let block_object_map = unsafe{ (**block).get_object_map() };
                self.object_map_backup.extend(block_object_map.into_iter());
            }
        }

        for block in self.all_blocks.iter_mut() {
            unsafe{ (**block).clear_line_counts(); }
            unsafe{ (**block).clear_object_map(); }
        }
    }

    pub fn complete_immix_collection(&mut self) {
        if cfg!(feature = "valgrind") {
            let mut object_map = HashSet::new();
            for block in self.all_blocks.iter_mut() {
                let block_object_map = unsafe{ (**block).get_object_map() };
                object_map.extend(block_object_map.into_iter());
            }
            for &object in self.object_map_backup.difference(&object_map) {
                valgrind_freelike!(object);
            }
            self.object_map_backup.clear();
        }
    }
}

impl Collector {
    fn sweep_all_blocks(&mut self) -> (RingBuf<*mut BlockInfo>, RingBuf<*mut BlockInfo>){
        let mut block_allocator = self.block_allocator.borrow_mut();
        let mut unavailable_blocks = RingBuf::new();
        let mut recyclable_blocks = RingBuf::new();
        let mut evac_headroom = RingBuf::new();
        for block in self.all_blocks.drain() {
            if unsafe{ (*block).is_empty() } {
                if cfg!(feature = "valgrind") {
                    let block_object_map = unsafe{ (*block).get_object_map() };
                    for &object in block_object_map.iter() {
                        valgrind_freelike!(object);
                    }
                }
                unsafe{ (*block).reset() ;}

                // XXX We should not use a constant here, but something that
                // XXX changes dynamically (see rcimmix: MAX heuristic).
                if evac_headroom.len() < EVAC_HEADROOM {
                    debug!("Buffer free block {:p} for evacuation", block);
                    evac_headroom.push_back(block);
                } else {
                    debug!("Return block {:p} to global block allocator", block);
                    block_allocator.return_block(block);
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
                        recyclable_blocks.push_back(block);
                    }
                }
            }
        }
        self.all_blocks = unavailable_blocks;
        return (recyclable_blocks, evac_headroom);
    }

    fn establish_hole_threshhold(&self, evac_headroom: usize) -> u8 {
        let mut available_histogram : VecMap<u8> = VecMap::with_capacity(NUM_LINES_PER_BLOCK);
        for block in self.all_blocks.iter() {
            let (holes, free_lines) = unsafe{ (**block).count_holes_and_available_lines() };
            if available_histogram.contains_key(&(holes as usize)) {
                if let Some(val) = available_histogram.get_mut(&(holes as usize)) {
                    *val += free_lines;
                }
            } else { available_histogram.insert(holes as usize, free_lines); }
        }
        let mut required_lines = 0 as u8;
        let mut available_lines = (evac_headroom
                                   * (NUM_LINES_PER_BLOCK - 1)) as u8;

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
