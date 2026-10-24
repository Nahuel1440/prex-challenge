//! Integration tests of `POST /new_client` through the real routes and error contract.

mod common;

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
    web::Data,
    App, Error,
};
use prex_processor::{application::ClientService, http, infrastructure::UnwiredCutOffFileWriter};
use serde_json::{json, Value};
use std::rc::Rc;

const CONCURRENT_REQUESTS: usize = 20;

fn valid_body_with_document(document_number: &str) -> Value {
    json!({
        "client_name": "Ana Pérez",
        "birth_date": "1990-05-17",
        "document_number": document_number,
        "country": "ar",
    })
}

fn body_with(field: &str, value: Value) -> Value {
    let mut body = valid_body_with_document("12345678");
    body[field] = value;
    body
}

async fn running_app() -> impl Service<Request, Response = ServiceResponse, Error = Error> {
    test::init_service(
        App::new()
            .app_data(Data::new(ClientService::new(
                common::writer_that_is_never_used(),
                common::FIRST_FILE_COUNTER,
            )))
            .configure(http::configure::<UnwiredCutOffFileWriter>),
    )
    .await
}

async fn post_new_client<S>(app: &S, body: &Value) -> (StatusCode, Value)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::post()
        .uri("/new_client")
        .set_json(body)
        .to_request();
    let response = test::call_service(app, request).await;
    let status = response.status();

    (status, test::read_body_json(response).await)
}

async fn assert_invalid_request(body: Value) {
    let app = running_app().await;

    let (status, response_body) = post_new_client(&app, &body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response_body["code"], "INVALID_REQUEST");
}

#[actix_web::test]
async fn new_client_returns_201_with_client_id() {
    let app = running_app().await;

    let (status, body) = post_new_client(&app, &valid_body_with_document("12345678")).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body, json!({ "client_id": 1 }));
}

#[actix_web::test]
async fn new_client_assigns_ids_1_2_3_in_order() {
    let app = running_app().await;
    let mut ids = Vec::new();

    for document_number in ["1", "2", "3"] {
        let (_, body) = post_new_client(&app, &valid_body_with_document(document_number)).await;
        ids.push(body["client_id"].clone());
    }

    assert_eq!(ids, [json!(1), json!(2), json!(3)]);
}

#[actix_web::test]
async fn new_client_duplicate_normalized_document_returns_409_document_already_registered() {
    let app = running_app().await;
    post_new_client(&app, &valid_body_with_document("12.345.678")).await;

    let (status, body) = post_new_client(&app, &valid_body_with_document("12345678")).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "DOCUMENT_ALREADY_REGISTERED");
}

#[actix_web::test]
async fn new_client_blank_name_returns_400_invalid_request() {
    assert_invalid_request(body_with("client_name", json!("   "))).await;
}

#[actix_web::test]
async fn new_client_name_over_100_chars_returns_400_invalid_request() {
    assert_invalid_request(body_with("client_name", json!("a".repeat(101)))).await;
}

#[actix_web::test]
async fn new_client_future_birth_date_returns_400_invalid_request() {
    assert_invalid_request(body_with("birth_date", json!("2999-01-01"))).await;
}

#[actix_web::test]
async fn new_client_unknown_country_returns_400_invalid_request() {
    assert_invalid_request(body_with("country", json!("ZZ"))).await;
}

#[actix_web::test]
async fn new_client_document_with_other_symbols_returns_400_invalid_request() {
    assert_invalid_request(body_with("document_number", json!("123/45"))).await;
}

#[actix_web::test]
async fn new_client_malformed_json_returns_400_invalid_request_with_json_body() {
    let app = running_app().await;
    let request = test::TestRequest::post()
        .uri("/new_client")
        .insert_header(("content-type", "application/json"))
        .set_payload("{ not json")
        .to_request();

    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["code"], "INVALID_REQUEST");
}

#[actix_web::test]
async fn new_client_missing_field_returns_400_invalid_request() {
    let mut body = valid_body_with_document("12345678");
    body.as_object_mut().unwrap().remove("country");

    assert_invalid_request(body).await;
}

#[actix_web::test]
async fn new_client_accepts_lowercase_country() {
    let app = running_app().await;

    let (status, _) = post_new_client(&app, &body_with("country", json!("ar"))).await;

    assert_eq!(status, StatusCode::CREATED);
}

#[actix_web::test]
async fn error_response_body_has_code_and_message() {
    let app = running_app().await;

    let (_, body) = post_new_client(&app, &body_with("country", json!("ZZ"))).await;

    assert_eq!(
        body,
        json!({ "code": "INVALID_REQUEST", "message": "country must be an ISO 3166-1 alpha-2 code" })
    );
}

#[actix_web::test]
async fn new_client_concurrent_requests_same_document_yield_exactly_one_201() {
    let app = Rc::new(running_app().await);

    let requests: Vec<_> = (0..CONCURRENT_REQUESTS)
        .map(|_| {
            let app = Rc::clone(&app);
            tokio::task::spawn_local(async move {
                post_new_client(&*app, &valid_body_with_document("12.345.678"))
                    .await
                    .0
            })
        })
        .collect();
    let mut created = 0;
    for request in requests {
        created += usize::from(request.await.unwrap() == StatusCode::CREATED);
    }

    assert_eq!(created, 1);
}

#[actix_web::test]
async fn new_client_document_empty_after_normalization_returns_400_invalid_request() {
    assert_invalid_request(body_with("document_number", json!(" . - "))).await;
}

#[actix_web::test]
async fn new_client_birth_date_in_another_format_returns_400_invalid_request() {
    assert_invalid_request(body_with("birth_date", json!("17/05/1990"))).await;
}
