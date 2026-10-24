//! Client entity.

use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;

use super::{Amount, BirthDate, ClientFieldError, ClientName, Country, DocumentNumber};

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

/// A movement that would leave the balance outside the range of `Decimal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceOverflow;

impl fmt::Display for BalanceOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the movement would overflow the client balance")
    }
}

impl std::error::Error for BalanceOverflow {}

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

    /// Adds `amount` to the balance; the balance is unchanged if it would overflow.
    pub fn credit(&mut self, amount: Amount) -> Result<(), BalanceOverflow> {
        self.balance = self
            .balance
            .checked_add(amount.value())
            .ok_or(BalanceOverflow)?;

        Ok(())
    }

    /// Subtracts `amount` from the balance, which may go negative; unchanged if it would overflow.
    pub fn debit(&mut self, amount: Amount) -> Result<(), BalanceOverflow> {
        self.balance = self
            .balance
            .checked_sub(amount.value())
            .ok_or(BalanceOverflow)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::str::FromStr;

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

    fn client_with_balance(balance: Decimal) -> Client {
        let new_client = NewClient::parse(
            "Ana",
            NaiveDate::from_ymd_opt(1990, 5, 17).unwrap(),
            "123",
            "AR",
            NaiveDate::from_ymd_opt(2026, 10, 24).unwrap(),
        )
        .unwrap();
        let mut client = Client::open(ClientId::new(1), new_client);
        client.balance = balance;
        client
    }

    fn amount(text: &str) -> Amount {
        Amount::try_from(Decimal::from_str(text).unwrap()).unwrap()
    }

    #[test]
    fn credit_adds_amount_to_balance() {
        let mut client = client_with_balance(Decimal::ZERO);

        client.credit(amount("100.50")).unwrap();

        assert_eq!(client.balance, Decimal::from_str("100.50").unwrap());
    }

    #[test]
    fn debit_subtracts_amount_from_balance() {
        let mut client = client_with_balance(Decimal::from_str("100.50").unwrap());

        client.debit(amount("30")).unwrap();

        assert_eq!(client.balance, Decimal::from_str("70.50").unwrap());
    }

    #[test]
    fn debit_below_zero_leaves_negative_balance() {
        let mut client = client_with_balance(Decimal::from_str("70.50").unwrap());

        client.debit(amount("1000")).unwrap();

        assert_eq!(client.balance, Decimal::from_str("-929.50").unwrap());
    }

    #[test]
    fn credit_past_decimal_max_fails_with_balance_overflow_and_keeps_balance() {
        let mut client = client_with_balance(Decimal::MAX);

        let error = client.credit(amount("1")).unwrap_err();

        assert_eq!(error, BalanceOverflow);
        assert_eq!(client.balance, Decimal::MAX);
    }

    #[test]
    fn debit_below_decimal_min_fails_with_balance_overflow_and_keeps_balance() {
        let mut client = client_with_balance(Decimal::MIN);

        let error = client.debit(amount("1")).unwrap_err();

        assert_eq!(error, BalanceOverflow);
        assert_eq!(client.balance, Decimal::MIN);
    }
}
