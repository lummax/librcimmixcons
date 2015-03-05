// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

mod rc_collector;
mod immix_collector;

use self::rc_collector::RCCollector;
use self::immix_collector::ImmixCollector;

use spaces::immix_space::BlockInfo;
use spaces::immix_space::ImmixSpace;
use spaces::large_object_space::LargeObjectSpace;

use std::collections::{HashSet, VecMap};
use std::num::Int;

use constants::{NUM_LINES_PER_BLOCK, USE_RC_COLLECTOR, USE_EVACUATION,
                EVAC_HEADROOM, CICLE_TRIGGER_THRESHHOLD, EVAC_TRIGGER_THRESHHOLD};
use gc_object::GCObjectRef;
use spaces::CollectionType;

/// The `Collector` is a composition of the collection implementations
/// `RCCollector` and `ImmixCollector`.
///
/// It manages performs common tasks and manages the cooperation between the
/// two collectors.
pub struct Collector {
    /// The reference counting collector.
    rc_collector: RCCollector,

    /// A buffer to store all managed blocks during collection.
    all_blocks: Vec<*mut BlockInfo>,

    /// A global backup of the object map.
    ///
    /// This is only used, if the library is compiled with `Valgrind` support
    /// and needed for accurate marking of addresses as `malloclike_block` and
    /// `freelike_block`.
    object_map_backup: HashSet<GCObjectRef>,

    /// The mark histogram used during collection to calculate the required
    /// space for evacuation.
    mark_histogram: VecMap<usize>,
}

impl Collector {
    /// Create a new `Collector`.
    pub fn new() -> Collector {
        return Collector {
            rc_collector: RCCollector::new(),
            all_blocks: Vec::new(),
            object_map_backup: HashSet::new(),
            mark_histogram: VecMap::with_capacity(NUM_LINES_PER_BLOCK),
        };
    }

    /// A write barrier for the given `object` used with the `RCCollector`.
    pub fn write_barrier(&mut self, object: GCObjectRef) {
        if USE_RC_COLLECTOR {
            self.rc_collector.write_barrier(object);
        }
    }

    /// Store the given blocks into the buffer for use during the collection.
    pub fn extend_all_blocks(&mut self, blocks: Vec<*mut BlockInfo>) {
        self.all_blocks.extend(blocks.into_iter());
    }

    /// Prepare a collection.
    ///
    /// This function decides if a evacuating and/or cycle collecting
    /// collection will be performed. If `evacuation` is set the collectors
    /// will try to evacuate. If `cycle_collect` is set the immix tracing
    /// collector will be used.
    pub fn prepare_collection(&mut self, evacuation: bool, cycle_collect: bool,
                              available_blocks: usize, total_blocks: usize,
                              evac_headroom: usize) -> CollectionType {
        let mut perform_evac = evacuation;

        let evac_threshhold = ((total_blocks as f32) * EVAC_TRIGGER_THRESHHOLD) as usize;
        let available_evac_blocks = available_blocks + evac_headroom;
        if evacuation || available_evac_blocks < evac_threshhold {
            let hole_threshhold = self.establish_hole_threshhold(evac_headroom);
            perform_evac = USE_EVACUATION && hole_threshhold > 0
                                          && hole_threshhold < NUM_LINES_PER_BLOCK;
            if perform_evac {
                debug!("Performing evacuation with hole_threshhold={} and evac_headroom={}",
                       hole_threshhold, evac_headroom);
                for block in self.all_blocks.iter_mut() {
                    unsafe{ (**block).set_evacuation_candidate(hole_threshhold); }
                }
            }
        }

        let perform_cycle_collect = if !cycle_collect {
            let cycle_theshold = ((total_blocks as f32) * CICLE_TRIGGER_THRESHHOLD) as usize;
            available_blocks < cycle_theshold
        } else { true };

        return match (USE_RC_COLLECTOR, perform_evac, perform_cycle_collect) {
            (true, false, false) => CollectionType::RCCollection,
            (true, true, false) => CollectionType::RCEvacCollection,
            (true, false, true) => CollectionType::ImmixCollection,
            (true, true, true) => CollectionType::ImmixEvacCollection,
            (false, false, _) => CollectionType::ImmixCollection,
            (false, true, _) => CollectionType::ImmixEvacCollection,
        }
    }

    /// Perform the collection.
    ///
    /// See `Spaces.collect() how it is called.`
    pub fn collect(&mut self, collection_type: &CollectionType,
                   roots: &[GCObjectRef], immix_space: &mut ImmixSpace,
                   large_object_space: &mut LargeObjectSpace, next_live_mark: bool) {

        if USE_RC_COLLECTOR {
            self.perform_rc_collection(collection_type, roots, immix_space,
                                       large_object_space);
        }

        if collection_type.is_immix() {
            self.perform_immix_collection(collection_type, roots,
                                          immix_space, next_live_mark);
        }
    }

    /// Perform the reference counting collection.
    fn perform_rc_collection(&mut self, collection_type: &CollectionType,
                             roots: &[GCObjectRef],
                             immix_space: &mut ImmixSpace,
                             large_object_space: &mut LargeObjectSpace) {
        if cfg!(feature = "valgrind") {
            for block in self.all_blocks.iter_mut() {
                let block_new_objects = unsafe{ (**block).get_new_objects() };
                self.object_map_backup.extend(block_new_objects.into_iter());
            }
        }

        for block in self.all_blocks.iter_mut() {
            unsafe{ (**block).remove_new_objects_from_map(); }
        }

        self.rc_collector.collect(collection_type, roots, immix_space, large_object_space);

        large_object_space.proccess_free_buffer();

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

    /// Perform the immix tracing collection.
    pub fn perform_immix_collection(&mut self, collection_type: &CollectionType,
                                    roots: &[GCObjectRef],
                                    immix_space: &mut ImmixSpace,
                                    next_live_mark: bool) {
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

        ImmixCollector::collect(collection_type, roots, immix_space, next_live_mark);

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

    /// Complete the collection.
    pub fn complete_collection(&mut self, collection_type: &CollectionType,
                               immix_space: &mut ImmixSpace,
                               large_object_space: &mut LargeObjectSpace) {
        self.mark_histogram.clear();
        let (recyclable_blocks, free_blocks) = self.sweep_all_blocks();
        immix_space.set_recyclable_blocks(recyclable_blocks);

        // XXX We should not use a constant here, but something that
        // XXX changes dynamically (see rcimmix: MAX heuristic).
        let evac_headroom = if USE_EVACUATION {
            EVAC_HEADROOM - immix_space.evac_headroom() } else { 0 };
        immix_space.extend_evac_headroom(free_blocks.iter().take(evac_headroom)
                                                    .map(|&b| b).collect());
        immix_space.return_blocks(free_blocks.iter().skip(evac_headroom)
                                             .map(|&b| b).collect());

        if collection_type.is_immix() {
            large_object_space.sweep()
        }
    }
}

impl Collector {
    /// Sweep all blocks in the buffer after the collection.
    ///
    /// This function returns a list of recyclable blocks and a list of free
    /// blocks.
    fn sweep_all_blocks(&mut self) -> (Vec<*mut BlockInfo>, Vec<*mut BlockInfo>){
        let mut unavailable_blocks = Vec::new();
        let mut recyclable_blocks = Vec::new();
        let mut free_blocks = Vec::new();
        for block in self.all_blocks.drain() {
            if unsafe{ (*block).is_empty() } {
                if cfg!(feature = "valgrind") {
                    let block_object_map = unsafe{ (*block).get_object_map() };
                    for &object in block_object_map.iter() {
                        valgrind_freelike!(object);
                    }
                }
                unsafe{ (*block).reset() ;}
                debug!("Push block {:p} into free_blocks", block);
                free_blocks.push(block);
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
                        unavailable_blocks.push(block);
                    },
                    _ => {
                        debug!("Push block {:p} into recyclable_blocks", block);
                        recyclable_blocks.push(block);
                    }
                }
            }
        }
        self.all_blocks = unavailable_blocks;
        return (recyclable_blocks, free_blocks);
    }

    /// Calculate how many holes a block needs to have to be selected as a
    /// evacuation candidate.
    fn establish_hole_threshhold(&self, evac_headroom: usize) -> usize {
        let mut available_histogram : VecMap<usize> = VecMap::with_capacity(NUM_LINES_PER_BLOCK);
        for block in self.all_blocks.iter() {
            let (holes, free_lines) = unsafe{ (**block).count_holes_and_available_lines() };
            if available_histogram.contains_key(&(holes as usize)) {
                if let Some(val) = available_histogram.get_mut(&(holes as usize)) {
                    *val += free_lines;
                }
            } else { available_histogram.insert(holes as usize, free_lines); }
        }
        let mut required_lines = 0;
        let mut available_lines = evac_headroom * (NUM_LINES_PER_BLOCK - 1);

        for threshold in (0..NUM_LINES_PER_BLOCK) {
            required_lines += *self.mark_histogram.get(&threshold).unwrap_or(&0);
            available_lines = available_lines.saturating_sub(*available_histogram.get(&threshold).unwrap_or(&0));
            if available_lines <= required_lines {
                return threshold;
            }
        }
        return NUM_LINES_PER_BLOCK;
    }
}
