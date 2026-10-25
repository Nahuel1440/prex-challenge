//! Client entity.

use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{Amount, BirthDate, ClientFieldError, ClientName, Country, DocumentNumber};

/// Identifier of a client, unique within one run of the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// A movement whose resulting balance cannot be represented exactly by `Decimal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceOverflow;

impl fmt::Display for BalanceOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the resulting balance cannot be represented exactly")
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

    /// Adds `amount` exactly; leaves the balance unchanged on overflow or precision loss.
    pub fn credit(&mut self, amount: Amount) -> Result<(), BalanceOverflow> {
        self.balance = exact_balance_sum(self.balance, amount.value())?;

        Ok(())
    }

    /// Subtracts `amount` exactly, allowing negative balances; unchanged if not representable.
    pub fn debit(&mut self, amount: Amount) -> Result<(), BalanceOverflow> {
        self.balance = exact_balance_sum(self.balance, -amount.value())?;

        Ok(())
    }

    /// Sets the balance back to zero, after the cut-off stored it.
    pub fn reset_balance(&mut self) {
        self.balance = Decimal::ZERO;
    }
}

/// Returns `balance + change` as checked `Decimal` arithmetic gives it, keeping its scale.
///
/// Fails with `BalanceOverflow` when the sum is out of range or differs from the exact sum,
/// because `Decimal` rounds silently once its 96-bit mantissa is full.
fn exact_balance_sum(balance: Decimal, change: Decimal) -> Result<Decimal, BalanceOverflow> {
    let checked_sum = balance.checked_add(change).ok_or(BalanceOverflow)?;
    let common_scale = balance.normalize().scale().max(change.normalize().scale());

    let balance_units = mantissa_at_scale(balance, common_scale).ok_or(BalanceOverflow)?;
    let change_units = mantissa_at_scale(change, common_scale).ok_or(BalanceOverflow)?;
    let exact_units = balance_units
        .checked_add(change_units)
        .ok_or(BalanceOverflow)?;

    if mantissa_at_scale(checked_sum, common_scale) != Some(exact_units) {
        return Err(BalanceOverflow);
    }

    Ok(checked_sum)
}

/// The value as an integer count of `10^-scale` units, or `None` if it needs more decimals or overflows.
fn mantissa_at_scale(value: Decimal, scale: u32) -> Option<i128> {
    let normalized = value.normalize();
    let factor = 10_i128.checked_pow(scale.checked_sub(normalized.scale())?)?;

    normalized.mantissa().checked_mul(factor)
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

    #[test]
    fn credit_precision_loss_fails_with_balance_overflow_and_keeps_balance() {
        for initial in [
            Decimal::MAX,
            Decimal::MIN,
            Decimal::from_str("792281625142643375935439503.35").unwrap(),
        ] {
            let mut client = client_with_balance(initial);

            let result = client.credit(amount("0.01"));

            assert_eq!(result, Err(BalanceOverflow), "initial balance: {initial}");
            assert_eq!(client.balance, initial);
        }
    }

    #[test]
    fn debit_precision_loss_fails_with_balance_overflow_and_keeps_balance() {
        for initial in [
            Decimal::MAX,
            Decimal::MIN,
            Decimal::from_str("-792281625142643375935439503.35").unwrap(),
        ] {
            let mut client = client_with_balance(initial);

            let result = client.debit(amount("0.01"));

            assert_eq!(result, Err(BalanceOverflow), "initial balance: {initial}");
            assert_eq!(client.balance, initial);
        }
    }

    #[test]
    fn credit_accepts_exact_high_magnitude_sums_carries_and_cancellation() {
        for (initial, movement, expected) in [
            (
                "792281625142643375935439503.34",
                "0.01",
                "792281625142643375935439503.35",
            ),
            (
                "792281625142643375935439503.35",
                "0.65",
                "792281625142643375935439504",
            ),
            (
                "79228162514264337593543950334",
                "1",
                "79228162514264337593543950335",
            ),
            (
                "-79228162514264337593543950335",
                "79228162514264337593543950335",
                "0",
            ),
        ] {
            let mut client = client_with_balance(Decimal::from_str(initial).unwrap());

            client.credit(amount(movement)).unwrap();

            assert_eq!(client.balance, Decimal::from_str(expected).unwrap());
        }
    }

    #[test]
    fn debit_accepts_exact_high_magnitude_differences_carries_and_cancellation() {
        for (initial, movement, expected) in [
            (
                "-792281625142643375935439503.34",
                "0.01",
                "-792281625142643375935439503.35",
            ),
            (
                "-792281625142643375935439503.35",
                "0.65",
                "-792281625142643375935439504",
            ),
            (
                "-79228162514264337593543950334",
                "1",
                "-79228162514264337593543950335",
            ),
            (
                "79228162514264337593543950335",
                "79228162514264337593543950335",
                "0",
            ),
        ] {
            let mut client = client_with_balance(Decimal::from_str(initial).unwrap());

            client.debit(amount(movement)).unwrap();

            assert_eq!(client.balance, Decimal::from_str(expected).unwrap());
        }
    }

    #[test]
    fn exact_movements_preserve_trailing_zero_scale() {
        let mut client = client_with_balance(Decimal::ZERO);

        client.credit(amount("72.000")).unwrap();
        assert_eq!(client.balance.to_string(), "72.000");

        client.debit(amount("1.0")).unwrap();
        assert_eq!(client.balance.to_string(), "71.000");
    }
}
