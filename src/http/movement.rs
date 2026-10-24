//! `POST /new_credit_transaction` and `POST /new_debit_transaction`.

use std::future::Future;
use std::str::FromStr;

use actix_web::{post, web, HttpResponse};
use rust_decimal::Decimal;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use serde_json::Number;

use crate::application::{AppError, ClientService};
use crate::domain::{Amount, ClientId};

/// Body of `POST /new_credit_transaction`.
#[derive(Debug, Deserialize)]
pub struct CreditRequest {
    client_id: ClientId,
    #[serde(deserialize_with = "deserialize_json_number")]
    credit_amount: Decimal,
}

/// Body of `POST /new_debit_transaction`.
#[derive(Debug, Deserialize)]
pub struct DebitRequest {
    client_id: ClientId,
    #[serde(deserialize_with = "deserialize_json_number")]
    debit_amount: Decimal,
}

/// Body of the 200 answer of both movements.
#[derive(Debug, Serialize)]
pub struct MovementResponse {
    client_id: ClientId,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    balance: Decimal,
}

/// Reads a JSON number into a `Decimal` digit for digit; strings and other types are rejected.
fn deserialize_json_number<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let number = Number::deserialize(deserializer)?;

    Decimal::from_str(&number.to_string()).map_err(D::Error::custom)
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
async fn answer_with_new_balance<Movement>(
    client_id: ClientId,
    requested_amount: Decimal,
    movement: impl FnOnce(Amount) -> Movement,
) -> Result<HttpResponse, AppError>
where
    Movement: Future<Output = Result<Decimal, AppError>>,
{
    let amount = Amount::try_from(requested_amount)?;
    let balance = movement(amount).await?;

    Ok(HttpResponse::Ok().json(MovementResponse { client_id, balance }))
}
