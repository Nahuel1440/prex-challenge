//! Placeholder writer until the file system adapter exists.

use std::io;

use crate::application::{CutOffFileWriter, CutOffLine};

/// A writer that always fails, so nothing is lost silently while no real writer is wired.
///
/// Replaced in slice 05b by the file system writer.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnwiredCutOffFileWriter;

impl CutOffFileWriter for UnwiredCutOffFileWriter {
    async fn write(&self, _file_name: &str, _lines: &[CutOffLine]) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}
