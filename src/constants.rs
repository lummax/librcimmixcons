// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

pub const HEAP_SIZE: usize = 1024 * 1024 * 1024;

pub const BLOCK_SIZE: usize = 32 * 1024;
pub const LINE_SIZE: usize = 256;
pub const NUM_LINES_PER_BLOCK: usize = BLOCK_SIZE / LINE_SIZE;

pub const EVAC_HEADROOM: usize = 5;

pub const CICLE_TRIGGER_THRESHHOLD: f32 = 0.01;
pub const EVAC_TRIGGER_THRESHHOLD: f32 = 0.01;
