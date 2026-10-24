//! Adapters that talk to the outside world, such as the file system.

mod fs_cut_off_file_writer;
mod next_file_counter;
mod stale_temp_files;

pub use fs_cut_off_file_writer::FsCutOffFileWriter;
pub use next_file_counter::{next_file_counter, next_file_counter_in_dir};
pub use stale_temp_files::remove_stale_temp_files;
