//! Why a client field was rejected.

use std::fmt;

use super::client_name::MAX_CLIENT_NAME_CHARS;

/// A client field that breaks a domain rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientFieldError {
    /// `client_name` is empty or only whitespace.
    EmptyName,
    /// `client_name` has more characters than the allowed maximum.
    NameTooLong,
    /// `birth_date` is after today.
    FutureBirthDate,
    /// `document_number` has no characters left after normalization.
    EmptyDocument,
    /// `document_number` has a character other than letters and digits.
    InvalidDocumentCharacter,
    /// `country` is not an ISO 3166-1 alpha-2 code.
    UnknownCountry,
}

impl fmt::Display for ClientFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyName => "client_name must not be empty",
            Self::NameTooLong => {
                return write!(
                    formatter,
                    "client_name must have at most {MAX_CLIENT_NAME_CHARS} characters"
                )
            }
            Self::FutureBirthDate => "birth_date must not be in the future",
            Self::EmptyDocument => "document_number must not be empty",
            Self::InvalidDocumentCharacter => "document_number must only have letters and digits",
            Self::UnknownCountry => "country must be an ISO 3166-1 alpha-2 code",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClientFieldError {}
