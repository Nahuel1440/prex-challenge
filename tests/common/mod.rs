//! Helpers shared by the integration tests of endpoints that never run a cut-off.

use prex_processor::infrastructure::UnwiredCutOffFileWriter;

/// Counter for tests that never run a cut-off, so any value works.
pub const FIRST_FILE_COUNTER: u64 = 1;

/// A writer for tests that never run a cut-off.
pub fn writer_that_is_never_used() -> UnwiredCutOffFileWriter {
    UnwiredCutOffFileWriter
}
