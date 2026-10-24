//! Helpers shared by the integration tests of endpoints that never run a cut-off.

use std::path::PathBuf;

use prex_processor::infrastructure::FsCutOffFileWriter;

/// Counter for tests that never run a cut-off, so any value works.
pub const FIRST_FILE_COUNTER: u64 = 1;

/// A writer pointing at a folder that does not exist: a cut-off by mistake would fail loudly.
pub fn writer_that_is_never_used() -> FsCutOffFileWriter {
    FsCutOffFileWriter::new(PathBuf::from("this_folder_does_not_exist"))
}
