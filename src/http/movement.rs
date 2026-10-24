//! `POST /new_credit_transaction` and `POST /new_debit_transaction`.

use std::future::Future;

use actix_web::{post, web, HttpResponse};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Number;

use crate::application::{AppError, ClientService};
use crate::domain::{Amount, AmountError, ClientId};

/// Body of `POST /new_credit_transaction`.
#[derive(Debug, Deserialize)]
pub struct CreditRequest {
    client_id: ClientId,
    credit_amount: Number,
}

/// Body of `POST /new_debit_transaction`.
#[derive(Debug, Deserialize)]
pub struct DebitRequest {
    client_id: ClientId,
    debit_amount: Number,
}

/// Body of the 200 answer of both movements.
#[derive(Debug, Serialize)]
pub struct MovementResponse {
    client_id: ClientId,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    balance: Decimal,
}

/// Turns a JSON number into an `Amount` digit for digit: nothing is rounded.
///
/// More precision than a `Decimal` holds is `INVALID_AMOUNT`; other numbers a `Decimal`
/// cannot hold (too large, exponent notation) are `INVALID_REQUEST`.
fn parse_amount(requested_amount: &Number) -> Result<Amount, AppError> {
    let decimal =
        Decimal::from_str_exact(&requested_amount.to_string()).map_err(|error| match error {
            rust_decimal::Error::Underflow => AmountError::TooManyDecimals.into(),
            other => AppError::InvalidRequest(other.to_string()),
        })?;

    Ok(Amount::try_from(decimal)?)
}

/// Credits the client and answers 200 with the new balance.
#[post("/new_credit_transaction")]
pub async fn new_credit_transaction(
    service: web::Data<ClientService>,
    request: web::Json<CreditRequest>,
) -> Result<HttpResponse, AppError> {
    let CreditRequest {
        client_id,
        credit_amount,
    } = request.into_inner();

    answer_with_new_balance(client_id, credit_amount, |amount| {
        service.credit(client_id, amount)
    })
    .await
}

/// Debits the client and answers 200 with the new balance, which may be negative.
#[post("/new_debit_transaction")]
pub async fn new_debit_transaction(
    service: web::Data<ClientService>,
    request: web::Json<DebitRequest>,
) -> Result<HttpResponse, AppError> {
    let DebitRequest {
        client_id,
        debit_amount,
    } = request.into_inner();

    answer_with_new_balance(client_id, debit_amount, |amount| {
        service.debit(client_id, amount)
    })
    .await
}

/// The flow both movements share: validate the amount, run the movement, answer the balance.
async fn answer_with_new_balance<MovementFuture>(
    client_id: ClientId,
    requested_amount: Number,
    movement: impl FnOnce(Amount) -> MovementFuture,
) -> Result<HttpResponse, AppError>
where
    MovementFuture: Future<Output = Result<Decimal, AppError>>,
{
    let amount = parse_amount(&requested_amount)?;
    let balance = movement(amount).await?;

    Ok(HttpResponse::Ok().json(MovementResponse { client_id, balance }))
}
