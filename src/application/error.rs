//! Application errors.

use std::fmt;

use crate::domain::ClientFieldError;

/// A use case that could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The request is malformed or has an invalid field; the text says what to fix.
    InvalidRequest(String),
    /// Another client already has this normalized document number.
    DocumentAlreadyRegistered,
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(reason) => formatter.write_str(reason),
            Self::DocumentAlreadyRegistered => {
                formatter.write_str("a client with this document_number already exists")
            }
        }
    }
}

impl std::error::Error for AppError {}

impl From<ClientFieldError> for AppError {
    fn from(error: ClientFieldError) -> Self {
        Self::InvalidRequest(error.to_string())
    }
}
