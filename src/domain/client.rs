//! Client entity.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;

use super::{BirthDate, ClientFieldError, ClientName, Country, DocumentNumber};

/// Identifier of a client, unique within one run of the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ClientId(u64);

impl ClientId {
    /// Wraps a raw number.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw number.
    pub fn value(self) -> u64 {
        self.0
    }
}

/// The validated data of a client that is not registered yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClient {
    /// Trimmed name.
    pub name: ClientName,
    /// Birth date, not after today.
    pub birth_date: BirthDate,
    /// Normalized document.
    pub document_number: DocumentNumber,
    /// Uppercase ISO country.
    pub country: Country,
}

impl NewClient {
    /// Validates every field; `today` is the UTC date given by the caller.
    pub fn parse(
        name: &str,
        birth_date: NaiveDate,
        document_number: &str,
        country: &str,
        today: NaiveDate,
    ) -> Result<Self, ClientFieldError> {
        Ok(Self {
            name: ClientName::parse(name)?,
            birth_date: BirthDate::parse(birth_date, today)?,
            document_number: DocumentNumber::parse(document_number)?,
            country: Country::parse(country)?,
        })
    }
}

/// A registered client with its balance since the last cut-off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    /// Identifier given at registration.
    pub id: ClientId,
    /// Validated personal data.
    pub profile: NewClient,
    /// Net balance since the last cut-off; starts at zero and can be negative.
    pub balance: Decimal,
}

impl Client {
    /// Registers `profile` under `id` with a zero balance.
    pub fn open(id: ClientId, profile: NewClient) -> Self {
        Self {
            id,
            profile,
            balance: Decimal::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn new_client_starts_with_zero_balance() {
        let new_client = NewClient::parse(
            "Ana",
            NaiveDate::from_ymd_opt(1990, 5, 17).unwrap(),
            "123",
            "AR",
            NaiveDate::from_ymd_opt(2026, 10, 24).unwrap(),
        )
        .unwrap();

        let client = Client::open(ClientId::new(1), new_client);

        assert_eq!(client.balance, Decimal::ZERO);
    }
}
