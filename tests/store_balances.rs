//! Integration tests of `POST /store_balances` with the real file system writer.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
    web::Data,
    App, Error,
};
use chrono::Utc;
use prex_processor::{
    application::{ClientService, CutOffFileWriter, CutOffLine},
    domain::ClientId,
    http,
    infrastructure::FsCutOffFileWriter,
};
use regex::Regex;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use tokio::sync::Notify;

/// A cut-off file name, `DDMMYYYY_N.DAT`.
const CUT_OFF_FILE_NAME_PATTERN: &str = r"^\d{8}_\d+\.DAT$";
/// Id of the first registered client.
const FIRST_CLIENT_ID: u64 = 1;
/// Id of the second registered client.
const SECOND_CLIENT_ID: u64 = 2;
/// Number of the first cut-off file of a service that starts on an empty folder.
const FIRST_FILE_COUNTER: u64 = 1;
/// The only text of a failed cut-off; it names no path and no system error.
const CUT_OFF_FAILED_MESSAGE: &str = "the cut-off file could not be stored";
const STORE_BALANCES_URI: &str = "/store_balances";

/// Answer of the endpoint: status and parsed JSON body.
struct Answer {
    status: StatusCode,
    body: Value,
}

fn service_writing_to(dir: &Path) -> Data<ClientService<FsCutOffFileWriter>> {
    Data::new(ClientService::new(
        FsCutOffFileWriter::new(dir.to_owned()),
        FIRST_FILE_COUNTER,
    ))
}

async fn running_app<W: CutOffFileWriter + 'static>(
    service: &Data<ClientService<W>>,
) -> impl Service<Request, Response = ServiceResponse, Error = Error> {
    test::init_service(
        App::new()
            .app_data(service.clone())
            .configure(http::configure::<W>),
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
        .uri(STORE_BALANCES_URI)
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

/// A folder that cannot receive files, because its path is a regular file.
fn unusable_cut_off_dir(parent: &Path) -> PathBuf {
    let path = parent.join("not_a_directory");
    fs::write(&path, "").unwrap();

    path
}

fn file_name_of(answer: &Answer) -> &str {
    answer.body["file_name"].as_str().unwrap()
}

fn is_cut_off_file_name(file_name: &str) -> bool {
    Regex::new(CUT_OFF_FILE_NAME_PATTERN)
        .unwrap()
        .is_match(file_name)
}

fn assert_file_counter(answer: &Answer, expected_counter: u64) {
    assert!(file_name_of(answer).ends_with(&format!("_{expected_counter}.DAT")));
}

#[actix_web::test]
async fn store_balances_returns_200_with_the_file_name() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;

    let answer = post_store_balances(&app).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert!(is_cut_off_file_name(file_name_of(&answer)));
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
async fn store_balances_names_the_file_with_todays_utc_date() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;

    let date_before_call = Utc::now().format("%d%m%Y").to_string();
    let answer = post_store_balances(&app).await;
    let date_after_call = Utc::now().format("%d%m%Y").to_string();

    let file_name = file_name_of(&answer);
    assert!(file_name.starts_with(&date_before_call) || file_name.starts_with(&date_after_call));
}

#[actix_web::test]
async fn second_store_balances_uses_the_next_counter() {
    let dir = tempfile::tempdir().unwrap();
    let app = running_app(&service_writing_to(dir.path())).await;

    let first = post_store_balances(&app).await;
    let second = post_store_balances(&app).await;

    assert_file_counter(&first, 1);
    assert_file_counter(&second, 2);
}

#[actix_web::test]
async fn store_balances_returns_500_cut_off_failed_without_leaking_the_path() {
    let parent = tempfile::tempdir().unwrap();
    let unusable_dir = unusable_cut_off_dir(parent.path());
    let app = running_app(&service_writing_to(&unusable_dir)).await;

    let answer = post_store_balances(&app).await;

    assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        answer.body,
        json!({ "code": "CUT_OFF_FAILED", "message": CUT_OFF_FAILED_MESSAGE })
    );
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
    assert_file_counter(&retried, 1);
    let content = fs::read_to_string(cut_off_dir.join(file_name_of(&retried))).unwrap();
    assert_eq!(content, "1 500.12\n2 -3000\n");
}

/// Pauses inside `write` until the test lets it go, so a cut-off can be caught mid-write.
struct GatedWriter {
    write_started: Arc<Notify>,
    write_released: Arc<Notify>,
    inner: FsCutOffFileWriter,
}

impl CutOffFileWriter for GatedWriter {
    async fn write(&self, file_name: &str, lines: &[CutOffLine]) -> std::io::Result<()> {
        self.write_started.notify_one();
        self.write_released.notified().await;

        self.inner.write(file_name, lines).await
    }
}

#[actix_web::test]
async fn store_balances_completes_when_the_caller_disconnects_mid_write() {
    let dir = tempfile::tempdir().unwrap();
    let write_started = Arc::new(Notify::new());
    let write_released = Arc::new(Notify::new());
    let service = Data::new(ClientService::new(
        GatedWriter {
            write_started: write_started.clone(),
            write_released: write_released.clone(),
            inner: FsCutOffFileWriter::new(dir.path().to_owned()),
        },
        FIRST_FILE_COUNTER,
    ));
    let app = running_app(&service).await;
    register_clients_with_movements(&app).await;
    let request = test::TestRequest::post()
        .uri(STORE_BALANCES_URI)
        .to_request();
    let mut in_flight_call = Box::pin(test::call_service(&app, request));

    tokio::select! {
        _ = &mut in_flight_call => panic!("the cut-off ended before the write was released"),
        () = write_started.notified() => {}
    }
    drop(in_flight_call);
    write_released.notify_one();

    // `find` takes the same lock as the cut-off, so it answers only after the orphaned
    // cut-off has finished.
    let first_client = service.find(ClientId::new(FIRST_CLIENT_ID)).await.unwrap();
    assert_eq!(first_client.balance, Decimal::ZERO);
    // Lets the write of the next cut-off through; the permit is kept until it waits.
    write_released.notify_one();
    let next_cut_off = post_store_balances(&app).await;
    assert_file_counter(&next_cut_off, 2);
}

/// Panics inside `write`, as a bug in an adapter would.
struct PanickingWriter;

impl CutOffFileWriter for PanickingWriter {
    async fn write(&self, _file_name: &str, _lines: &[CutOffLine]) -> std::io::Result<()> {
        panic!("the writer panicked");
    }
}

#[actix_web::test]
async fn store_balances_returns_500_cut_off_failed_when_the_cut_off_task_panics() {
    let service = Data::new(ClientService::new(PanickingWriter, FIRST_FILE_COUNTER));
    let app = running_app(&service).await;

    let answer = post_store_balances(&app).await;

    assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(answer.body["code"], "CUT_OFF_FAILED");
}
