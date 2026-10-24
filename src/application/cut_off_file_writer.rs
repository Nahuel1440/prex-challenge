//! The port the cut-off uses to store balances, and the data it carries.

use std::future::Future;

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::domain::ClientId;

/// The balance of one client at the moment of a cut-off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutOffLine {
    /// The client the balance belongs to.
    pub client_id: ClientId,
    /// Net balance since the previous cut-off; can be negative.
    pub balance: Decimal,
}

/// Stores the lines of one cut-off under a file name chosen by the application.
///
/// The adapter owns the line format and the place where the file lives.
pub trait CutOffFileWriter {
    /// Writes the whole file or nothing. Never overwrites an existing file.
    ///
    /// Declared with `impl Future + Send` instead of `async fn` so callers can rely on `Send`;
    /// implementations may still be written as `async fn write(...)`.
    fn write(
        &self,
        file_name: &str,
        lines: &[CutOffLine],
    ) -> impl Future<Output = std::io::Result<()>> + Send;
}

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
