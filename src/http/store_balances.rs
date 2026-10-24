//! `POST /store_balances`.

use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Serialize;

use crate::application::{AppError, ClientService, CutOffFileWriter};

/// Body of the 200 answer: the file the balances were stored in.
#[derive(Debug, Serialize)]
pub struct StoreBalancesResponse {
    file_name: String,
}

/// Stores every balance in a new cut-off file, resets them, and answers 200 with the file name.
pub async fn store_balances<W: CutOffFileWriter + 'static>(
    service: web::Data<ClientService<W>>,
) -> Result<HttpResponse, AppError> {
    let today = Utc::now().date_naive();
    let file_name = service.store_balances(today).await?;

    Ok(HttpResponse::Ok().json(StoreBalancesResponse { file_name }))
}
