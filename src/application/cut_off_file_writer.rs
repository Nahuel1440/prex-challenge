//! The port the cut-off uses to store balances, and the data it carries.

use std::future::Future;

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
/// The adapter owns the line format and the place where the file lives. It is `Send + Sync`
/// because the cut-off runs in its own task.
pub trait CutOffFileWriter: Send + Sync {
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
