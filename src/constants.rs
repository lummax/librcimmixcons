// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

// XXX Should be 32K at least
// XXX Getting the BlockInfo from the object pointer needs fixing if
// XXX BLOCK_SIZE != page size and the memory maps are not BLOCK_SIZE aligned.
pub const BLOCK_SIZE : usize = 4 * 1024;

pub const LINE_SIZE: usize = 256;
pub const NUM_LINES_PER_BLOCK: usize = BLOCK_SIZE / LINE_SIZE;

pub const EVAC_HEADROOM: usize = 5;
pub const BUFFER_BLOCK_COUNT: usize = 10;
