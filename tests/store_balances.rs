//! Integration tests of `POST /store_balances` with the real file system writer.

use std::fs;
use std::path::Path;

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
    web::Data,
    App, Error,
};
use prex_processor::{application::ClientService, http, infrastructure::FsCutOffFileWriter};
use regex::Regex;
use serde_json::Value;

/// A cut-off file name, `DDMMYYYY_N.DAT`.
const CUT_OFF_FILE_NAME_PATTERN: &str = r"^\d{8}_\d+\.DAT$";
/// Id of the first registered client.
const FIRST_CLIENT_ID: u64 = 1;
/// Id of the second registered client.
const SECOND_CLIENT_ID: u64 = 2;

/// Answer of the endpoint: status and parsed JSON body.
struct Answer {
    status: StatusCode,
    body: Value,
}

fn service_writing_to(dir: &Path) -> Data<ClientService<FsCutOffFileWriter>> {
    Data::new(ClientService::new(
        FsCutOffFileWriter::new(dir.to_owned()),
        1,
    ))
}

async fn running_app(
    service: &Data<ClientService<FsCutOffFileWriter>>,
) -> impl Service<Request, Response = ServiceResponse, Error = Error> {
    test::init_service(
        App::new()
            .app_data(service.clone())
            .configure(http::configure::<FsCutOffFileWriter>),
    )
    .await
}

async fn post_json<S>(app: &S, uri: &str, body_text: String)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::post()
        .uri(uri)
        .insert_header(("content-type", "application/json"))
        .set_payload(body_text)
        .to_request();
    let response = test::call_service(app, request).await;
    assert!(response.status().is_success());
}

async fn register_client<S>(app: &S, document_number: &str)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    post_json(
        app,
        "/new_client",
        format!(
            r#"{{"client_name":"Ana Pérez","birth_date":"1990-05-17","document_number":"{document_number}","country":"AR"}}"#
        ),
    )
    .await;
}

/// Registers two clients: the first ends with 500.12 and the second with -3000.
async fn register_clients_with_movements<S>(app: &S)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    register_client(app, "11111111").await;
    register_client(app, "22222222").await;
    post_json(
        app,
        "/new_credit_transaction",
        format!(r#"{{"client_id":{FIRST_CLIENT_ID},"credit_amount":500.12}}"#),
    )
    .await;
    post_json(
        app,
        "/new_debit_transaction",
        format!(r#"{{"client_id":{SECOND_CLIENT_ID},"debit_amount":3000}}"#),
    )
    .await;
}

async fn post_store_balances<S>(app: &S) -> Answer
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::post()
        .uri("/store_balances")
        .to_request();
    let response = test::call_service(app, request).await;
    let status = response.status();

    Answer {
        status,
        body: test::read_body_json(response).await,
    }
}

async fn balance_of<S>(app: &S, client_id: u64) -> Value
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::get()
        .uri(&format!("/client_balance?user_id={client_id}"))
        .to_request();
    let body: Value = test::call_and_read_body_json(app, request).await;

    body["balance"].clone()
}

fn file_name_of(answer: &Answer) -> &str {
    answer.body["file_name"].as_str().unwrap()
}

#[actix_web::test]
async fn store_balances_returns_200_with_the_file_name() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;

    let answer = post_store_balances(&app).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert!(Regex::new(CUT_OFF_FILE_NAME_PATTERN)
        .unwrap()
        .is_match(file_name_of(&answer)));
}

#[actix_web::test]
async fn store_balances_writes_the_file_in_the_cut_off_dir() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;
    register_clients_with_movements(&app).await;

    let answer = post_store_balances(&app).await;

    let content = fs::read_to_string(dir.path().join(file_name_of(&answer))).unwrap();
    assert_eq!(content, "1 500.12\n2 -3000\n");
}

#[actix_web::test]
async fn store_balances_resets_every_balance_to_zero() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;
    register_clients_with_movements(&app).await;

    post_store_balances(&app).await;

    assert_eq!(balance_of(&app, FIRST_CLIENT_ID).await, 0);
    assert_eq!(balance_of(&app, SECOND_CLIENT_ID).await, 0);
}

#[actix_web::test]
async fn second_store_balances_uses_the_next_counter() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;

    let first = post_store_balances(&app).await;
    let second = post_store_balances(&app).await;

    assert!(file_name_of(&first).ends_with("_1.DAT"));
    assert!(file_name_of(&second).ends_with("_2.DAT"));
}

/// A folder that cannot receive files, because its path is a regular file.
fn unusable_cut_off_dir(parent: &Path) -> std::path::PathBuf {
    let path = parent.join("not_a_directory");
    fs::write(&path, "").unwrap();

    path
}

#[actix_web::test]
async fn store_balances_returns_500_cut_off_failed_without_leaking_the_path() {
    let parent = tempfile::tempdir().unwrap();
    let unusable_dir = unusable_cut_off_dir(parent.path());
    let app = running_app(&service_writing_to(&unusable_dir)).await;

    let answer = post_store_balances(&app).await;

    assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(answer.body["code"], "CUT_OFF_FAILED");
    assert!(!answer.body.to_string().contains("not_a_directory"));
}

#[actix_web::test]
async fn failed_store_balances_keeps_balances_and_the_next_success_reuses_the_counter() {
    let parent = tempfile::tempdir().unwrap();
    let cut_off_dir = parent.path().join("cut_off");
    let app = running_app(&service_writing_to(&cut_off_dir)).await;
    register_clients_with_movements(&app).await;

    let failed = post_store_balances(&app).await;
    fs::create_dir(&cut_off_dir).unwrap();
    let retried = post_store_balances(&app).await;

    assert_eq!(failed.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(file_name_of(&retried).ends_with("_1.DAT"));
    let content = fs::read_to_string(cut_off_dir.join(file_name_of(&retried))).unwrap();
    assert_eq!(content, "1 500.12\n2 -3000\n");
}
