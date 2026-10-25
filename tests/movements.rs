//! Integration tests of `POST /new_credit_transaction` and `POST /new_debit_transaction`.

mod common;

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test, Error,
};
use common::Answer;

const CREDIT_URI: &str = "/new_credit_transaction";
const DEBIT_URI: &str = "/new_debit_transaction";
/// Largest `Decimal` value, written out because JSON numbers keep every digit.
const DECIMAL_MAX: &str = "79228162514264337593543950335";
const DECIMAL_MIN: &str = "-79228162514264337593543950335";

async fn app_with_one_client() -> impl Service<Request, Response = ServiceResponse, Error = Error> {
    let app = common::running_app().await;
    common::post_json_expecting_success(
        &app,
        "/new_client",
        r#"{"client_name":"Ana Pérez","birth_date":"1990-05-17","document_number":"12345678","country":"AR"}"#,
    )
    .await;

    app
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

async fn set_initial_balance<S>(app: &S, balance: &str)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let (uri, body) = match balance.strip_prefix('-') {
        Some(amount) => (DEBIT_URI, debit_body(1, amount)),
        None => (CREDIT_URI, credit_body(1, balance)),
    };
    common::post_json_expecting_success(app, uri, &body).await;
}

async fn assert_current_balance<S>(app: &S, expected: &str)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::get()
        .uri("/client_balance?user_id=1")
        .to_request();
    let answer = Answer::from_response(test::call_service(app, request).await).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body()["balance"].to_string(), expected);
}

#[actix_web::test]
async fn credit_returns_200_with_client_id_and_new_balance() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, "100.50")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "100.50"));
}

#[actix_web::test]
async fn credit_then_debit_returns_running_balance() {
    let app = app_with_one_client().await;
    common::post_json(&app, CREDIT_URI, &credit_body(1, "100.50")).await;

    let answer = common::post_json(&app, DEBIT_URI, &debit_body(1, "30")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "70.50"));
}

#[actix_web::test]
async fn debit_below_zero_returns_200_with_negative_balance() {
    let app = app_with_one_client().await;
    common::post_json(&app, CREDIT_URI, &credit_body(1, "70.50")).await;

    let answer = common::post_json(&app, DEBIT_URI, &debit_body(1, "1000")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "-929.50"));
}

#[actix_web::test]
async fn credit_amount_zero_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, "0")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_negative_amount_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, "-5")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_amount_with_three_decimals_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, "1.001")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn debit_amount_zero_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, DEBIT_URI, &debit_body(1, "0")).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_amount_as_string_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, r#""100.50""#)).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn credit_amount_missing_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, r#"{"client_id":1}"#).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn debit_amount_as_string_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, DEBIT_URI, &debit_body(1, r#""30""#)).await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn credit_unknown_client_returns_404_client_not_found() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(42, "10")).await;

    assert_error(&answer, StatusCode::NOT_FOUND, "CLIENT_NOT_FOUND");
}

#[actix_web::test]
async fn debit_unknown_client_returns_404_client_not_found() {
    let app = app_with_one_client().await;

    let answer = common::post_json(&app, DEBIT_URI, &debit_body(42, "10")).await;

    assert_error(&answer, StatusCode::NOT_FOUND, "CLIENT_NOT_FOUND");
}

#[actix_web::test]
async fn credit_overflow_returns_422_balance_overflow_and_balance_unchanged() {
    let app = app_with_one_client().await;
    common::post_json_expecting_success(&app, CREDIT_URI, &credit_body(1, DECIMAL_MAX)).await;

    let overflow = common::post_json(&app, CREDIT_URI, &credit_body(1, "1")).await;
    let balance_after = common::post_json(&app, DEBIT_URI, &debit_body(1, "1")).await;

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

    let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, "12345678901234567.89")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body_text, balance_body(1, "12345678901234567.89"));
}

#[actix_web::test]
async fn credit_amount_with_more_than_28_decimals_returns_400_invalid_amount() {
    let app = app_with_one_client().await;

    let answer = common::post_json(
        &app,
        CREDIT_URI,
        &credit_body(1, "1.00000000000000000000000000001"),
    )
    .await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_AMOUNT");
}

#[actix_web::test]
async fn credit_client_id_as_string_returns_400_invalid_request() {
    let app = app_with_one_client().await;

    let answer = common::post_json(
        &app,
        CREDIT_URI,
        r#"{"client_id":"abc","credit_amount":10}"#,
    )
    .await;

    assert_error(&answer, StatusCode::BAD_REQUEST, "INVALID_REQUEST");
}

#[actix_web::test]
async fn debit_below_decimal_min_returns_422_balance_overflow_and_balance_unchanged() {
    let app = app_with_one_client().await;
    common::post_json_expecting_success(&app, DEBIT_URI, &debit_body(1, DECIMAL_MAX)).await;

    let overflow = common::post_json(&app, DEBIT_URI, &debit_body(1, "1")).await;
    let balance_after = common::post_json(&app, CREDIT_URI, &credit_body(1, "1")).await;

    assert_error(
        &overflow,
        StatusCode::UNPROCESSABLE_ENTITY,
        "BALANCE_OVERFLOW",
    );
    assert_eq!(
        balance_after.body_text,
        balance_body(1, "-79228162514264337593543950334")
    );
}

#[actix_web::test]
async fn credit_precision_loss_returns_422_and_keeps_balance() {
    for initial in [DECIMAL_MAX, DECIMAL_MIN, "792281625142643375935439503.35"] {
        let app = app_with_one_client().await;
        set_initial_balance(&app, initial).await;

        let answer = common::post_json(&app, CREDIT_URI, &credit_body(1, "0.01")).await;

        assert_error(
            &answer,
            StatusCode::UNPROCESSABLE_ENTITY,
            "BALANCE_OVERFLOW",
        );
        assert_current_balance(&app, initial).await;
    }
}

#[actix_web::test]
async fn debit_precision_loss_returns_422_and_keeps_balance() {
    for initial in [DECIMAL_MAX, DECIMAL_MIN, "-792281625142643375935439503.35"] {
        let app = app_with_one_client().await;
        set_initial_balance(&app, initial).await;

        let answer = common::post_json(&app, DEBIT_URI, &debit_body(1, "0.01")).await;

        assert_error(
            &answer,
            StatusCode::UNPROCESSABLE_ENTITY,
            "BALANCE_OVERFLOW",
        );
        assert_current_balance(&app, initial).await;
    }
}

#[actix_web::test]
async fn exact_cent_movements_near_the_precision_boundary_return_200() {
    let app = app_with_one_client().await;
    set_initial_balance(&app, "792281625142643375935439503.34").await;

    let credited = common::post_json(&app, CREDIT_URI, &credit_body(1, "0.01")).await;
    let debited = common::post_json(&app, DEBIT_URI, &debit_body(1, "0.01")).await;

    assert_eq!(credited.status, StatusCode::OK);
    assert_eq!(
        credited.body_text,
        balance_body(1, "792281625142643375935439503.35")
    );
    assert_eq!(debited.status, StatusCode::OK);
    assert_eq!(
        debited.body_text,
        balance_body(1, "792281625142643375935439503.34")
    );
}

#[actix_web::test]
async fn exact_movements_keep_trailing_zeros_in_the_response() {
    let app = app_with_one_client().await;

    let credited = common::post_json(&app, CREDIT_URI, &credit_body(1, "72.000")).await;
    let debited = common::post_json(&app, DEBIT_URI, &debit_body(1, "1.0")).await;

    assert_eq!(credited.status, StatusCode::OK);
    assert_eq!(credited.body_text, balance_body(1, "72.000"));
    assert_eq!(debited.status, StatusCode::OK);
    assert_eq!(debited.body_text, balance_body(1, "71.000"));
}
