//! Integration tests of `POST /new_credit_transaction` and `POST /new_debit_transaction`.

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test,
    web::Data,
    App, Error,
};
use prex_processor::{application::ClientService, http};
use serde_json::{json, Value};

const CREDIT_URI: &str = "/new_credit_transaction";
const DEBIT_URI: &str = "/new_debit_transaction";
/// Largest `Decimal` value, written out because JSON numbers keep every digit.
const DECIMAL_MAX: &str = "79228162514264337593543950335";

/// Answer of a movement: status and the body exactly as sent on the wire.
struct Answer {
    status: StatusCode,
    body_text: String,
}

impl Answer {
    fn body(&self) -> Value {
        serde_json::from_str(&self.body_text).unwrap()
    }
}

async fn app_with_one_client() -> impl Service<Request, Response = ServiceResponse, Error = Error> {
    let app = test::init_service(
        App::new()
            .app_data(Data::new(ClientService::new()))
            .configure(http::configure),
    )
    .await;
    let register = test::TestRequest::post()
        .uri("/new_client")
        .set_json(json!({
            "client_name": "Ana Pérez",
            "birth_date": "1990-05-17",
            "document_number": "12345678",
            "country": "AR",
        }))
        .to_request();
    test::call_service(&app, register).await;

    app
}

async fn post_raw<S>(app: &S, uri: &str, body_text: &str) -> Answer
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::post()
        .uri(uri)
        .insert_header(("content-type", "application/json"))
        .set_payload(body_text.to_owned())
        .to_request();
    let response = test::call_service(app, request).await;
    let status = response.status();
    let body_bytes = test::read_body(response).await;

    Answer {
        status,
        body_text: String::from_utf8(body_bytes.to_vec()).unwrap(),
    }
}

fn credit_body(client_id: u64, amount: &str) -> String {
    format!(r#"{{"client_id":{client_id},"credit_amount":{amount}}}"#)
}

fn debit_body(client_id: u64, amount: &str) -> String {
    format!(r#"{{"client_id":{client_id},"debit_amount":{amount}}}"#)
}

fn balance_body(client_id: u64, balance: &str) -> String {
    format!(r#"{{"client_id":{client_id},"balance":{balance}}}"#)
}

fn assert_error(answer: &Answer, status: StatusCode, code: &str) {
    assert_eq!(answer.status, status);
    assert_eq!(answer.body()["code"], code);
}

#[actix_web::test]
async fn credit_returns_200_with_client_id_and_new_balance() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(1, "100.50")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "100.50"));
}

#[actix_web::test]
async fn credit_then_debit_returns_running_balance() {
    let app = app_with_one_client().await;
    post_raw(&app, CREDIT_URI, &credit_body(1, "100.50")).await;

    let answer = post_raw(&app, DEBIT_URI, &debit_body(1, "30")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "70.50"));
}

#[actix_web::test]
async fn debit_below_zero_returns_200_with_negative_balance() {
    let app = app_with_one_client().await;
    post_raw(&app, CREDIT_URI, &credit_body(1, "70.50")).await;

    let answer = post_raw(&app, DEBIT_URI, &debit_body(1, "1000")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "-929.50"));
}

#[actix_web::test]
async fn credit_amount_zero_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(1, "0")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_negative_amount_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(1, "-5")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_amount_with_three_decimals_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(1, "1.001")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn debit_amount_zero_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, DEBIT_URI, &debit_body(1, "0")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_amount_as_string_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(1, r#""100.50""#)).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn credit_amount_missing_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, r#"{"client_id":1}"#).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn debit_amount_as_string_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, DEBIT_URI, &debit_body(1, r#""30""#)).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn credit_unknown_client_returns_404_client_not_found() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(42, "10")).await;

    assert_error(&answer, StatusCode::NOT_FOUND, "CLIENT_NOT_FOUND");
}

#[actix_web::test]
async fn debit_unknown_client_returns_404_client_not_found() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, DEBIT_URI, &debit_body(42, "10")).await;

    assert_error(&answer, StatusCode::NOT_FOUND, "CLIENT_NOT_FOUND");
}

#[actix_web::test]
async fn credit_overflow_returns_422_balance_overflow_and_balance_unchanged() {
    let app = app_with_one_client().await;
    post_raw(&app, CREDIT_URI, &credit_body(1, DECIMAL_MAX)).await;

    let overflow = post_raw(&app, CREDIT_URI, &credit_body(1, "1")).await;
    let balance_after = post_raw(&app, DEBIT_URI, &debit_body(1, "1")).await;

    assert_error(
        &overflow,
        StatusCode::UNPROCESSABLE_ENTITY,
        "BALANCE_OVERFLOW",
    );
    assert_eq!(
        balance_after.body_text,
        balance_body(1, "79228162514264337593543950334")
    );
}

#[actix_web::test]
async fn large_amount_round_trips_without_precision_loss() {
    let app = app_with_one_client().await;

    let answer = post_raw(&app, CREDIT_URI, &credit_body(1, "12345678901234567.89")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "12345678901234567.89"));
}
