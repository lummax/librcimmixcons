// XXX Should be 32K at least
// XXX Getting the BlockInfo from the object pointer needs fixing if
// XXX BLOCK_SIZE != page size and the memory maps are not BLOCK_SIZE aligned.
pub const BLOCK_SIZE : uint = 4 * 1024;

pub const LINE_SIZE: uint = 256;
pub const NUM_LINES_PER_BLOCK: uint = BLOCK_SIZE / LINE_SIZE;
