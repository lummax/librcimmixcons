// Copyright (c) <2015> <lummax>
// Licensed under MIT (http://opensource.org/licenses/MIT)

/// The size of the heap in bytes.
pub const HEAP_SIZE: usize = 1024 * 1024 * 1024;

/// The size of a block in bytes.
pub const BLOCK_SIZE: usize = 32 * 1024;

/// The size of a line in bytes.
pub const LINE_SIZE: usize = 256;

/// The number of lines per block.
pub const NUM_LINES_PER_BLOCK: usize = BLOCK_SIZE / LINE_SIZE;

/// Objects smaller than MEDIUM_OBJECT are allocated with the
/// `NormalAllocator`, otherwise the `OverflowAllocator` is used.
pub const MEDIUM_OBJECT: usize = LINE_SIZE;

/// Objects larger than LARGE_OBJECT are allocated using the `LargeObjectSpace`.
pub const LARGE_OBJECT: usize = 8 * 1024;

/// Whether the reference counting collector should be used. Otherwise only
/// the tracing immix collector will be utilized.
pub const USE_RC_COLLECTOR: bool = true;

/// Ratio when to trigger cycle collection.
pub const CICLE_TRIGGER_THRESHHOLD: f32 = 0.01;

/// Whether evacuation should be used or not.
pub const USE_EVACUATION: bool = true;

/// The number of blocks stored into the `EvacAllocator` for evacuation.
pub const EVAC_HEADROOM: usize = 5;

/// Ratio when to trigger evacuation collection.
pub const EVAC_TRIGGER_THRESHHOLD: f32 = 0.01;
