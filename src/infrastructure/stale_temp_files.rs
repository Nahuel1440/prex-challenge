//! Cleans up the temp files a crashed cut-off may have left in the folder.

use std::fs;
use std::io;
use std::path::Path;

use super::fs_cut_off_file_writer::is_temp_file_name;

/// Deletes every temp file in `dir`; run it at startup, before any cut-off can be running.
///
/// A temp file is left behind only by a crash. Cut-off files and other names are kept.
pub fn remove_stale_temp_files(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let is_temp = entry.file_name().to_str().is_some_and(is_temp_file_name);

        if is_temp {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn startup_removes_stale_temp_files_and_keeps_cut_off_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in [".tmp-24102026_2.DAT.tmp", "24102026_1.DAT", "notes.txt"] {
            fs::write(dir.path().join(name), "1 10\n").unwrap();
        }

        remove_stale_temp_files(dir.path()).unwrap();

        let mut remaining: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        remaining.sort();
        assert_eq!(remaining, ["24102026_1.DAT", "notes.txt"]);
    }
}
