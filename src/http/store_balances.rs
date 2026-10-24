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
///
/// The cut-off runs in its own task: if the client disconnects, Actix drops this handler but
/// the task still finishes, so the file is never published without the balances being reset.
pub async fn store_balances<W: CutOffFileWriter + 'static>(
    service: web::Data<ClientService<W>>,
) -> Result<HttpResponse, AppError> {
    let today = Utc::now().date_naive();
    let service = service.into_inner();
    let cut_off = tokio::spawn(async move { service.store_balances(today).await });
    let file_name = cut_off.await.map_err(|cause| {
        tracing::error!(%cause, "cut-off task ended without a result");
        AppError::CutOffFailed
    })??;

    Ok(HttpResponse::Ok().json(StoreBalancesResponse { file_name }))
}
