//! Liveness endpoint for CI smoke tests and container probes.

use actix_web::{get, HttpResponse};
use serde_json::json;

/// Answers 200 as long as the process can serve requests.
#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok" }))
}
