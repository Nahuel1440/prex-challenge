//! Stores a cut-off as a text file in a folder.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::application::{CutOffFileWriter, CutOffLine};

/// Start of the name a file has while it is being written; no cut-off file name starts with it.
const TEMP_FILE_PREFIX: &str = ".tmp-";
/// End of the name a file has while it is being written.
const TEMP_FILE_SUFFIX: &str = ".tmp";

/// True for a name a file only has while it is being written.
pub(super) fn is_temp_file_name(file_name: &str) -> bool {
    file_name.starts_with(TEMP_FILE_PREFIX) && file_name.ends_with(TEMP_FILE_SUFFIX)
}

/// Writes each cut-off as a file in one folder, one `ID BALANCE` line per client.
///
/// The file appears complete or not at all: it is written under a temp name, flushed to disk
/// and then hard-linked to its final name. A hard link fails if the name is taken, so an
/// existing cut-off file is never replaced.
#[derive(Debug, Clone)]
pub struct FsCutOffFileWriter {
    dir: PathBuf,
}

impl FsCutOffFileWriter {
    /// A writer that puts its files in `dir`. The caller creates the folder, as `main` does at startup.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl CutOffFileWriter for FsCutOffFileWriter {
    async fn write(&self, file_name: &str, lines: &[CutOffLine]) -> io::Result<()> {
        let dir = self.dir.clone();
        let file_name = file_name.to_owned();
        let content = file_content(lines);

        tokio::task::spawn_blocking(move || publish_file(&dir, &file_name, &content))
            .await
            .map_err(io::Error::other)?
    }
}

/// The text of the file: `{id} {balance}\n` per line, balances without trailing zeros.
fn file_content(lines: &[CutOffLine]) -> String {
    lines
        .iter()
        .map(|line| format!("{} {}\n", line.client_id.value(), line.balance.normalize()))
        .collect()
}

/// Makes `content` appear in `dir` under `file_name`, all at once.
///
/// A temp file left by a crash is removed first, because it may still be a link to a
/// published file.
fn publish_file(dir: &Path, file_name: &str, content: &str) -> io::Result<()> {
    let temp_path = dir.join(format!("{TEMP_FILE_PREFIX}{file_name}{TEMP_FILE_SUFFIX}"));
    let final_path = dir.join(file_name);

    remove_temp_file(&temp_path);
    let linked =
        write_and_sync(&temp_path, content).and_then(|()| fs::hard_link(&temp_path, &final_path));
    remove_temp_file(&temp_path);

    match &linked {
        Ok(()) => sync_dir(dir),
        Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => {
            tracing::error!(%file_name, "a cut-off file with this name already exists; an operator must act");
        }
        Err(_) => {}
    }

    linked
}

/// Flushes the folder entry of the new file. A failure only warns: the file is published.
///
/// No test covers the failure: a folder that opens but cannot be flushed cannot be built
/// in a portable way.
fn sync_dir(dir: &Path) {
    if let Err(cause) = File::open(dir).and_then(|folder| folder.sync_all()) {
        tracing::warn!(%cause, ?dir, "cut-off folder could not be flushed to disk");
    }
}

/// Removes the temp file on every outcome. A failure only warns: it never changes the result,
/// because after the link the cut-off file is already published.
fn remove_temp_file(temp_path: &Path) {
    match fs::remove_file(temp_path) {
        Ok(()) => {}
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => {}
        Err(cause) => {
            tracing::warn!(%cause, ?temp_path, "temp file of a cut-off could not be removed");
        }
    }
}

/// Creates the new file at `path` with `content` and flushes it to disk.
///
/// Fails if `path` exists, so it never writes through a link to another file.
fn write_and_sync(path: &Path, content: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ClientId;
    use rust_decimal::Decimal;
    use std::fs;
    use std::str::FromStr;

    fn line(client_id: u64, balance: &str) -> CutOffLine {
        CutOffLine {
            client_id: ClientId::new(client_id),
            balance: Decimal::from_str(balance).unwrap(),
        }
    }

    fn file_names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();

        names
    }

    #[tokio::test]
    async fn writes_one_line_per_client_as_id_space_balance() {
        let dir = tempfile::tempdir().unwrap();
        let writer = FsCutOffFileWriter::new(dir.path().to_owned());

        writer
            .write("24102026_1.DAT", &[line(1, "500.12"), line(2, "-3000.00")])
            .await
            .unwrap();

        let content = fs::read_to_string(dir.path().join("24102026_1.DAT")).unwrap();
        assert_eq!(content, "1 500.12\n2 -3000\n");
    }

    #[tokio::test]
    async fn writes_an_empty_file_when_there_are_no_clients() {
        let dir = tempfile::tempdir().unwrap();
        let writer = FsCutOffFileWriter::new(dir.path().to_owned());

        writer.write("24102026_1.DAT", &[]).await.unwrap();

        let content = fs::read_to_string(dir.path().join("24102026_1.DAT")).unwrap();
        assert_eq!(content, "");
    }

    #[tokio::test]
    async fn never_overwrites_an_existing_cut_off_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("24102026_1.DAT"), "1 10\n").unwrap();
        let writer = FsCutOffFileWriter::new(dir.path().to_owned());

        let outcome = writer.write("24102026_1.DAT", &[line(1, "99")]).await;

        assert_eq!(outcome.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        let content = fs::read_to_string(dir.path().join("24102026_1.DAT")).unwrap();
        assert_eq!(content, "1 10\n");
    }

    #[tokio::test]
    async fn leaves_no_temp_file_after_a_successful_write() {
        let dir = tempfile::tempdir().unwrap();
        let writer = FsCutOffFileWriter::new(dir.path().to_owned());

        writer
            .write("24102026_1.DAT", &[line(1, "5")])
            .await
            .unwrap();

        assert_eq!(file_names_in(dir.path()), ["24102026_1.DAT"]);
    }

    #[tokio::test]
    async fn leaves_no_temp_file_after_a_failed_write() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("24102026_1.DAT"), "1 10\n").unwrap();
        let writer = FsCutOffFileWriter::new(dir.path().to_owned());

        writer
            .write("24102026_1.DAT", &[line(1, "99")])
            .await
            .unwrap_err();

        assert_eq!(file_names_in(dir.path()), ["24102026_1.DAT"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_never_modifies_a_file_linked_from_a_stale_temp() {
        let dir = tempfile::tempdir().unwrap();
        let published = dir.path().join("24102026_1.DAT");
        fs::write(&published, "1 10\n").unwrap();
        fs::hard_link(&published, dir.path().join(".tmp-24102026_2.DAT.tmp")).unwrap();
        let writer = FsCutOffFileWriter::new(dir.path().to_owned());

        writer
            .write("24102026_2.DAT", &[line(1, "99")])
            .await
            .unwrap();

        assert_eq!(fs::read_to_string(&published).unwrap(), "1 10\n");
        let content = fs::read_to_string(dir.path().join("24102026_2.DAT")).unwrap();
        assert_eq!(content, "1 99\n");
    }

    #[tokio::test]
    async fn fails_when_the_directory_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_directory = dir.path().join("occupied");
        fs::write(&not_a_directory, "").unwrap();
        let writer = FsCutOffFileWriter::new(not_a_directory);

        let outcome = writer.write("24102026_1.DAT", &[line(1, "5")]).await;

        assert!(outcome.is_err());
    }
}
