// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod block_info;
mod block_allocator;
mod allocator;
mod collector;

pub use self::collector::ImmixCollector;
pub use self::collector::RCCollector;

use self::block_info::BlockInfo;

use std::collections::{RingBuf, HashSet, VecMap};
use std::{mem, ptr};

use constants::{BLOCK_SIZE, NUM_LINES_PER_BLOCK, EVAC_HEADROOM,
                CICLE_TRIGGER_THRESHHOLD, EVAC_TRIGGER_THRESHHOLD};
use gc_object::{GCRTTI, GCObject, GCObjectRef};

pub struct ImmixSpace {
    allocator: allocator::Allocator,
    all_blocks: RingBuf<*mut BlockInfo>,
    object_map_backup: HashSet<GCObjectRef>,
    mark_histogram: VecMap<u8>,
    current_live_mark: bool,
    perform_evac: bool,
}

impl ImmixSpace {
    pub fn new() -> ImmixSpace {
        return ImmixSpace {
            allocator: allocator::Allocator::new(),
            all_blocks: RingBuf::new(),
            object_map_backup: HashSet::new(),
            mark_histogram: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
            current_live_mark: false,
            perform_evac: false,
        };
    }

    pub fn set_gc_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_in_space(object), "set_gc_object() on invalid space");
        unsafe{ (*self.get_block_ptr(object)).set_gc_object(object); }
    }

    pub fn unset_gc_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_in_space(object), "unset_gc_object() on invalid space");
        unsafe{ (*self.get_block_ptr(object)).unset_gc_object(object); }
    }

    pub fn is_gc_object(&mut self, object: GCObjectRef) -> bool {
        if self.is_in_space(object) {
            return unsafe{ (*self.get_block_ptr(object)).is_gc_object(object) };
        }
        return false;
    }

    pub fn is_in_space(&self, object: GCObjectRef) -> bool {
        return self.allocator.is_in_space(object);
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
        if let Some(object) = self.allocator.allocate(size, self.perform_evac) {
            unsafe { ptr::write(object, GCObject::new(rtti, self.current_live_mark)); }
            self.set_gc_object(object);
            self.set_new_object(object);
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
        if let Some(new_object) = self.allocator.allocate(size, self.perform_evac) {
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
        self.all_blocks = self.allocator.get_all_blocks();

        let available_blocks = self.allocator.block_allocator().available_blocks();
        let total_blocks = self.allocator.block_allocator().total_blocks();

        let evac_threshhold = ((total_blocks as f32) * EVAC_TRIGGER_THRESHHOLD) as usize;
        let available_evac_blocks = available_blocks + self.allocator.evac_headroom();
        if evacuation || available_evac_blocks < evac_threshhold {
            let hole_threshhold = self.establish_hole_threshhold();
            self.perform_evac = hole_threshhold > 0
                && hole_threshhold < NUM_LINES_PER_BLOCK as u8;
            if self.perform_evac {
                debug!("Performing evacuation with hole_threshhold={} and evac_headroom={}",
                       hole_threshhold, self.allocator.evac_headroom());
                for block in self.all_blocks.iter_mut() {
                    unsafe{ (**block).set_evacuation_candidate(hole_threshhold); }
                }
            }
        }

        if !cycle_collect {
            let cycle_theshold = ((total_blocks as f32) * CICLE_TRIGGER_THRESHHOLD) as usize;
            return self.allocator.block_allocator().available_blocks() < cycle_theshold;
        }
        return true;
    }

    pub fn complete_collection(&mut self) {
        self.mark_histogram.clear();
        self.perform_evac = false;
        self.sweep_all_blocks();
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
        self.current_live_mark = !self.current_live_mark;

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

impl ImmixSpace {
    unsafe fn get_block_ptr(&mut self, object: GCObjectRef) -> *mut BlockInfo {
        let block_offset = object as usize % BLOCK_SIZE;
        let block = mem::transmute((object as *mut u8).offset(-(block_offset as isize)));
        debug!("Block for object {:p}: {:p} with offset: {}", object, block, block_offset);
        return block;
    }

    fn set_new_object(&mut self, object: GCObjectRef) {
        debug_assert!(self.is_in_space(object), "set_new_object() on invalid space");
        unsafe{ (*self.get_block_ptr(object)).set_new_object(object); }
    }

    fn sweep_all_blocks(&mut self) {
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
                    self.allocator.block_allocator().return_block(block);
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
        self.allocator.set_unavailable_blocks(unavailable_blocks);
        self.allocator.set_recyclable_blocks(recyclable_blocks);
        self.allocator.extend_evac_headroom(evac_headroom);
    }

    fn establish_hole_threshhold(&self) -> u8 {
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
        let mut available_lines = (self.allocator.evac_headroom()
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
