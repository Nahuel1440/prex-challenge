//! The name of a cut-off file, `DDMMYYYY_N.DAT`.

use chrono::{Datelike, NaiveDate};

/// The name of the cut-off file of `date` (UTC) with sequence number `file_counter`.
///
/// Looks like `24102026_1.DAT`: day, month, four-digit year, then the counter.
pub(super) fn cut_off_file_name(date: NaiveDate, file_counter: u64) -> String {
    format!(
        "{:02}{:02}{:04}_{}.DAT",
        date.day(),
        date.month(),
        date.year(),
        file_counter
    )
}
