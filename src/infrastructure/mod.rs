//! Adapters that talk to the outside world, such as the file system.

mod fs_cut_off_file_writer;
mod next_file_counter;

pub use fs_cut_off_file_writer::FsCutOffFileWriter;
pub use next_file_counter::{next_file_counter, next_file_counter_in_dir};
