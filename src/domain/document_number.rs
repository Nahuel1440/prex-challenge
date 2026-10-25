//! Document number rule.

use super::ClientFieldError;

/// A document number without separators, in uppercase, with ASCII letters and digits only.
///
/// Two spellings of the same document (`12.345.678` and `12345678`) become equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentNumber(String);

impl DocumentNumber {
    /// Removes dots, dashes and whitespace, then uppercases; leading zeros stay.
    pub fn parse(raw: &str) -> Result<Self, ClientFieldError> {
        let normalized: String = raw
            .chars()
            .filter(|character| !is_separator(*character))
            .map(|character| character.to_ascii_uppercase())
            .collect();

        if normalized.is_empty() {
            return Err(ClientFieldError::EmptyDocument);
        }
        if !normalized.chars().all(is_uppercase_ascii_alphanumeric) {
            return Err(ClientFieldError::InvalidDocumentCharacter);
        }

        Ok(Self(normalized))
    }

    /// The normalized document.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Dots, dashes and whitespace separate groups of digits and are dropped.
fn is_separator(character: char) -> bool {
    character == '.' || character == '-' || character.is_whitespace()
}

/// Runs after uppercasing, so lowercase letters are never seen here.
fn is_uppercase_ascii_alphanumeric(character: char) -> bool {
    character.is_ascii_uppercase() || character.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ClientFieldError;

    #[test]
    fn document_number_strips_dots_dashes_and_whitespace() {
        assert_eq!(
            DocumentNumber::parse(" 12.345-678 9 ").unwrap().as_str(),
            "123456789"
        );
    }

    #[test]
    fn document_number_is_uppercased() {
        assert_eq!(DocumentNumber::parse("ab12").unwrap().as_str(), "AB12");
    }

    #[test]
    fn document_number_keeps_leading_zeros() {
        assert_eq!(DocumentNumber::parse("00123").unwrap().as_str(), "00123");
    }

    #[test]
    fn document_number_rejects_empty_after_normalization() {
        assert_eq!(
            DocumentNumber::parse(" . - ").unwrap_err(),
            ClientFieldError::EmptyDocument
        );
    }

    #[test]
    fn document_number_rejects_non_ascii_letters_that_uppercase_to_ascii() {
        let error = DocumentNumber::parse("ß1").unwrap_err();

        assert_eq!(error, ClientFieldError::InvalidDocumentCharacter);
    }

    #[test]
    fn document_number_rejects_other_symbols() {
        let error = DocumentNumber::parse("123/45").unwrap_err();

        assert_eq!(error, ClientFieldError::InvalidDocumentCharacter);
    }
}
