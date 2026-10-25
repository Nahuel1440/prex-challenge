//! `POST /new_client`.

use actix_web::{web, HttpResponse};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::application::{AppError, ClientRegistration, ClientService, CutOffFileWriter};
use crate::domain::ClientId;

/// Body of `POST /new_client`.
#[derive(Debug, Deserialize)]
pub struct NewClientRequest {
    client_name: String,
    birth_date: NaiveDate,
    document_number: String,
    country: String,
}

/// Body of the 201 answer.
#[derive(Debug, Serialize)]
pub struct NewClientResponse {
    client_id: ClientId,
}

impl From<NewClientRequest> for ClientRegistration {
    fn from(request: NewClientRequest) -> Self {
        Self {
            client_name: request.client_name,
            birth_date: request.birth_date,
            document_number: request.document_number,
            country: request.country,
        }
    }
}

/// Registers a client and answers 201 with its id.
pub async fn new_client<W: CutOffFileWriter + 'static>(
    service: web::Data<ClientService<W>>,
    request: web::Json<NewClientRequest>,
) -> Result<HttpResponse, AppError> {
    let today = Utc::now().date_naive();
    let client_id = service.register(request.into_inner().into(), today).await?;

    Ok(HttpResponse::Created().json(NewClientResponse { client_id }))
}
