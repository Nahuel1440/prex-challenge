//! Pure business rules: no I/O, no web framework.

mod amount;
mod birth_date;
mod client;
mod client_name;
mod country;
mod document_number;
mod error;

pub use amount::{Amount, AmountError};
pub use birth_date::BirthDate;
pub use client::{Client, ClientId, NewClient};
pub use client_name::ClientName;
pub use country::Country;
pub use document_number::DocumentNumber;
pub use error::ClientFieldError;
