//! A writer for tests: remembers every call instead of touching the disk.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use super::{CutOffFileWriter, CutOffLine};

/// One call to `write`: the file name and the lines it received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedWrite {
    /// Name the application chose for the file.
    pub file_name: String,
    /// Lines in the order the application sent them.
    pub lines: Vec<CutOffLine>,
}

/// Records the calls it gets; fails every call while told to.
#[derive(Debug, Default)]
pub struct FakeCutOffFileWriter {
    recorded_writes: Mutex<Vec<RecordedWrite>>,
    failing: AtomicBool,
}

impl FakeCutOffFileWriter {
    /// Every call made so far, in order; a failed call is not recorded.
    pub fn recorded_writes(&self) -> Vec<RecordedWrite> {
        self.recorded_writes.lock().unwrap().clone()
    }

    /// Makes every following call fail until `succeed_writes`.
    pub fn fail_writes(&self) {
        self.failing.store(true, Ordering::SeqCst);
    }

    /// Makes the following calls succeed again.
    pub fn succeed_writes(&self) {
        self.failing.store(false, Ordering::SeqCst);
    }
}

impl CutOffFileWriter for FakeCutOffFileWriter {
    async fn write(&self, file_name: &str, lines: &[CutOffLine]) -> io::Result<()> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(io::Error::other("the fake writer was told to fail"));
        }
        self.recorded_writes.lock().unwrap().push(RecordedWrite {
            file_name: file_name.to_owned(),
            lines: lines.to_vec(),
        });

        Ok(())
    }
}
