//! The error contract: how an `AppError` becomes an HTTP response.

use actix_web::{
    error::{JsonPayloadError, QueryPayloadError, ResponseError},
    http::StatusCode,
    HttpRequest, HttpResponse,
};
use serde::Serialize;

use crate::application::AppError;

/// JSON body of every error response.
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    /// Stable machine-readable code, such as `INVALID_REQUEST`.
    pub code: &'static str,
    /// Text for a person; never has paths or system errors.
    pub message: String,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "INVALID_REQUEST",
            Self::DocumentAlreadyRegistered => "DOCUMENT_ALREADY_REGISTERED",
            Self::InvalidAmount(_) => "INVALID_AMOUNT",
            Self::ClientNotFound(_) => "CLIENT_NOT_FOUND",
            Self::BalanceOverflow => "BALANCE_OVERFLOW",
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::DocumentAlreadyRegistered => StatusCode::CONFLICT,
            Self::InvalidAmount(_) => StatusCode::BAD_REQUEST,
            Self::ClientNotFound(_) => StatusCode::NOT_FOUND,
            Self::BalanceOverflow => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ApiErrorResponse {
            code: self.code(),
            message: self.to_string(),
        })
    }
}

/// Turns a body that is not valid JSON, or does not fit the DTO, into `INVALID_REQUEST`.
pub fn invalid_json_body(error: JsonPayloadError, _request: &HttpRequest) -> actix_web::Error {
    AppError::InvalidRequest(error.to_string()).into()
}

/// Turns a missing or non-integer query parameter into `INVALID_REQUEST`.
pub fn invalid_query_string(error: QueryPayloadError, _request: &HttpRequest) -> actix_web::Error {
    AppError::InvalidRequest(error.to_string()).into()
}
