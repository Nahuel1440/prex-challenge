//! `GET /client_balance`.

use actix_web::{web, HttpResponse};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::application::{AppError, ClientService, ClientView, CutOffFileWriter};
use crate::domain::ClientId;

/// Query string of `GET /client_balance`; the PDF calls the client id `user_id`.
#[derive(Debug, Deserialize)]
pub struct ClientBalanceQuery {
    user_id: u64,
}

/// Body of the 200 answer: the client data and its current balance.
#[derive(Debug, Serialize)]
pub struct ClientBalanceResponse {
    client_id: ClientId,
    client_name: String,
    birth_date: NaiveDate,
    document_number: String,
    country: String,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    balance: Decimal,
}

impl From<ClientView> for ClientBalanceResponse {
    fn from(view: ClientView) -> Self {
        Self {
            client_id: view.client_id,
            client_name: view.client_name,
            birth_date: view.birth_date,
            document_number: view.document_number,
            country: view.country,
            balance: view.balance,
        }
    }
}

/// Answers 200 with the client data and balance, or 404 if the id is unknown.
pub async fn client_balance<W: CutOffFileWriter + 'static>(
    service: web::Data<ClientService<W>>,
    query: web::Query<ClientBalanceQuery>,
) -> Result<HttpResponse, AppError> {
    let client_id = ClientId::new(query.user_id);
    let view = service.find(client_id).await?;

    Ok(HttpResponse::Ok().json(ClientBalanceResponse::from(view)))
}
