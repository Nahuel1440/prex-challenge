//! Birth date rule.

use chrono::NaiveDate;

use super::ClientFieldError;

/// A birth date that is not after today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BirthDate(NaiveDate);

impl BirthDate {
    /// Accepts any date up to and including `today`; the caller owns the clock.
    pub fn parse(birth_date: NaiveDate, today: NaiveDate) -> Result<Self, ClientFieldError> {
        if birth_date > today {
            return Err(ClientFieldError::FutureBirthDate);
        }

        Ok(Self(birth_date))
    }

    /// The validated date.
    pub fn value(self) -> NaiveDate {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ClientFieldError;
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn birth_date_today_is_accepted() {
        let today = date(2026, 10, 24);

        assert_eq!(BirthDate::parse(today, today).unwrap().value(), today);
    }

    #[test]
    fn birth_date_in_the_past_is_accepted() {
        assert!(BirthDate::parse(date(1990, 5, 17), date(2026, 10, 24)).is_ok());
    }

    #[test]
    fn birth_date_in_the_future_is_rejected() {
        let error = BirthDate::parse(date(2026, 10, 25), date(2026, 10, 24)).unwrap_err();

        assert_eq!(error, ClientFieldError::FutureBirthDate);
    }
}
