//! Client name rule.

use super::ClientFieldError;

/// Most characters a client name may have.
pub(super) const MAX_CLIENT_NAME_CHARS: usize = 100;

/// A trimmed, non-empty client name of at most `MAX_CLIENT_NAME_CHARS` characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientName(String);

impl ClientName {
    /// Trims the text and checks its length in characters, not bytes.
    pub fn parse(raw: &str) -> Result<Self, ClientFieldError> {
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return Err(ClientFieldError::EmptyName);
        }
        if trimmed.chars().count() > MAX_CLIENT_NAME_CHARS {
            return Err(ClientFieldError::NameTooLong);
        }

        Ok(Self(trimmed.to_owned()))
    }

    /// The trimmed name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ClientFieldError;

    #[test]
    fn client_name_is_trimmed_before_storing() {
        assert_eq!(
            ClientName::parse("  Ana Pérez \n").unwrap().as_str(),
            "Ana Pérez"
        );
    }

    #[test]
    fn client_name_rejects_empty_or_blank() {
        assert_eq!(
            ClientName::parse("").unwrap_err(),
            ClientFieldError::EmptyName
        );
        assert_eq!(
            ClientName::parse(" \t ").unwrap_err(),
            ClientFieldError::EmptyName
        );
    }

    #[test]
    fn client_name_accepts_exactly_100_characters() {
        assert!(ClientName::parse(&"a".repeat(MAX_CLIENT_NAME_CHARS)).is_ok());
    }

    #[test]
    fn client_name_rejects_more_than_100_characters() {
        let error = ClientName::parse(&"a".repeat(MAX_CLIENT_NAME_CHARS + 1)).unwrap_err();

        assert_eq!(error, ClientFieldError::NameTooLong);
    }

    #[test]
    fn client_name_counts_unicode_characters_not_bytes() {
        assert!(ClientName::parse(&"ñ".repeat(MAX_CLIENT_NAME_CHARS)).is_ok());
    }
}
