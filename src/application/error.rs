//! Application errors.

use std::fmt;

use crate::domain::{AmountError, BalanceOverflow, ClientFieldError, ClientId};

/// A use case that could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The request is malformed or has an invalid field; the text says what to fix.
    InvalidRequest(String),
    /// Another client already has this normalized document number.
    DocumentAlreadyRegistered,
    /// The movement amount is zero, negative or has too many decimals.
    InvalidAmount(AmountError),
    /// No client has this id.
    ClientNotFound(ClientId),
    /// The movement would exceed the range or exact precision of `Decimal`.
    BalanceOverflow,
    /// The cut-off file could not be stored; balances and counter were left unchanged.
    ///
    /// Carries no cause on purpose: the cause is logged, never shown to the issuer.
    CutOffFailed,
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(reason) => formatter.write_str(reason),
            Self::DocumentAlreadyRegistered => {
                formatter.write_str("a client with this document_number already exists")
            }
            Self::InvalidAmount(error) => write!(formatter, "{error}"),
            Self::ClientNotFound(client_id) => {
                write!(formatter, "client {} does not exist", client_id.value())
            }
            Self::BalanceOverflow => write!(formatter, "{BalanceOverflow}"),
            Self::CutOffFailed => formatter.write_str("the cut-off file could not be stored"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ClientFieldError> for AppError {
    fn from(error: ClientFieldError) -> Self {
        Self::InvalidRequest(error.to_string())
    }
}

impl From<AmountError> for AppError {
    fn from(error: AmountError) -> Self {
        Self::InvalidAmount(error)
    }
}

impl From<BalanceOverflow> for AppError {
    fn from(_overflow: BalanceOverflow) -> Self {
        Self::BalanceOverflow
    }
}
