//! Helpers shared by the HTTP integration tests.
//!
//! Each test crate uses a different subset, so unused items are expected.
#![allow(dead_code)]

use std::path::PathBuf;

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
    web::Data,
    App, Error,
};
use prex_processor::{application::ClientService, http, infrastructure::FsCutOffFileWriter};
use serde_json::Value;

/// Counter for tests that never run a cut-off, so any value works.
const FIRST_FILE_COUNTER: u64 = 1;

/// A writer pointing at a folder that does not exist: a cut-off by mistake would fail loudly.
fn writer_that_is_never_used() -> FsCutOffFileWriter {
    FsCutOffFileWriter::new(PathBuf::from("this_folder_does_not_exist"))
}

/// Answer of an endpoint: status, content type and the body exactly as sent on the wire.
pub struct Answer {
    pub status: StatusCode,
    pub content_type: String,
    pub body_text: String,
}

impl Answer {
    /// Reads status, content type and body text out of a response.
    pub async fn from_response(response: ServiceResponse) -> Self {
        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default();
        let body_bytes = test::read_body(response).await;

        Self {
            status,
            content_type,
            body_text: String::from_utf8(body_bytes.to_vec()).unwrap(),
        }
    }

    /// The body parsed as JSON.
    pub fn body(&self) -> Value {
        serde_json::from_str(&self.body_text).unwrap()
    }
}

/// An app with every route registered, whose cut-off writer must never be called.
pub async fn running_app() -> impl Service<Request, Response = ServiceResponse, Error = Error> {
    test::init_service(
        App::new()
            .app_data(Data::new(ClientService::new(
                writer_that_is_never_used(),
                FIRST_FILE_COUNTER,
            )))
            .configure(http::configure::<FsCutOffFileWriter>),
    )
    .await
}

/// Posts the body text as written, so `100.50` keeps its trailing zero.
pub async fn post_json<S>(app: &S, uri: &str, body_text: &str) -> Answer
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::post()
        .uri(uri)
        .insert_header(("content-type", "application/json"))
        .set_payload(body_text.to_owned())
        .to_request();

    Answer::from_response(test::call_service(app, request).await).await
}

/// Posts a body that setup needs to succeed, and fails the test if it does not.
pub async fn post_json_expecting_success<S>(app: &S, uri: &str, body_text: &str)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    assert!(post_json(app, uri, body_text).await.status.is_success());
}
