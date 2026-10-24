//! Read-only copy of a client.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::domain::{Client, ClientId};

/// What the issuer may read about a client, copied out of the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientView {
    /// Identifier given at registration.
    pub client_id: ClientId,
    /// Trimmed name.
    pub client_name: String,
    /// Birth date.
    pub birth_date: NaiveDate,
    /// Normalized document.
    pub document_number: String,
    /// Uppercase ISO country.
    pub country: String,
    /// Net balance since the last cut-off; can be negative.
    pub balance: Decimal,
}

impl From<&Client> for ClientView {
    fn from(client: &Client) -> Self {
        Self {
            client_id: client.id,
            client_name: client.profile.name.as_str().to_owned(),
            birth_date: client.profile.birth_date.value(),
            document_number: client.profile.document_number.as_str().to_owned(),
            country: client.profile.country.as_str().to_owned(),
            balance: client.balance,
        }
    }
}
