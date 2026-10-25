//! Money amount of one credit or debit.

use std::fmt;

use rust_decimal::Decimal;

/// Most decimal places an amount may have; more are rejected, never rounded.
pub const MAX_AMOUNT_DECIMALS: u32 = 2;

/// An amount that breaks a domain rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    /// The amount is zero or negative.
    NotPositive,
    /// The amount has more decimal places than allowed.
    TooManyDecimals,
}

impl fmt::Display for AmountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPositive => formatter.write_str("amount must be greater than 0"),
            Self::TooManyDecimals => write!(
                formatter,
                "amount must have at most {MAX_AMOUNT_DECIMALS} decimal places"
            ),
        }
    }
}

impl std::error::Error for AmountError {}

/// A strictly positive money value with at most two decimal places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(Decimal);

impl Amount {
    /// The validated value.
    pub fn value(self) -> Decimal {
        self.0
    }
}

impl TryFrom<Decimal> for Amount {
    type Error = AmountError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        if value <= Decimal::ZERO {
            return Err(AmountError::NotPositive);
        }
        if value.normalize().scale() > MAX_AMOUNT_DECIMALS {
            return Err(AmountError::TooManyDecimals);
        }

        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn decimal(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    #[test]
    fn amount_accepts_positive_value_with_two_decimals() {
        let amount = Amount::try_from(decimal("100.50")).unwrap();

        assert_eq!(amount.value(), decimal("100.50"));
    }

    #[test]
    fn amount_accepts_whole_number() {
        assert!(Amount::try_from(decimal("30")).is_ok());
    }

    #[test]
    fn amount_accepts_trailing_zero_decimals() {
        assert!(Amount::try_from(decimal("1.500")).is_ok());
    }

    #[test]
    fn amount_rejects_zero() {
        assert_eq!(
            Amount::try_from(Decimal::ZERO).unwrap_err(),
            AmountError::NotPositive
        );
    }

    #[test]
    fn amount_rejects_negative_value() {
        assert_eq!(
            Amount::try_from(decimal("-5")).unwrap_err(),
            AmountError::NotPositive
        );
    }

    #[test]
    fn amount_rejects_more_than_two_decimals() {
        assert_eq!(
            Amount::try_from(decimal("1.001")).unwrap_err(),
            AmountError::TooManyDecimals
        );
    }
}
