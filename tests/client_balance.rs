//! Integration tests of `GET /client_balance`.

mod common;

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::StatusCode,
    test, Error,
};
use common::Answer;

/// Id of the first registered client, Ana.
const ANA_ID: u64 = 1;

/// Registers Ana with a messy document and a lowercase country; she gets `ANA_ID`.
async fn register_ana<S>(app: &S)
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    common::post_json_expecting_success(
        app,
        "/new_client",
        r#"{"client_name":"Ana Pérez","birth_date":"1990-05-17","document_number":"12.345.678","country":"ar"}"#,
    )
    .await;
}

async fn get_client_balance<S>(app: &S, query: &str) -> Answer
where
    S: Service<Request, Response = ServiceResponse, Error = Error>,
{
    let request = test::TestRequest::get()
        .uri(&format!("/client_balance{query}"))
        .to_request();

    Answer::from_response(test::call_service(app, request).await).await
}

/// A bad `user_id` must give the JSON error contract, not Actix plain text.
async fn assert_invalid_request(query: &str) {
    let app = common::running_app().await;

    let answer = get_client_balance(&app, query).await;

    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    assert!(answer.content_type.starts_with("application/json"));
    assert_eq!(answer.body()["code"], "INVALID_REQUEST");
}

#[actix_web::test]
async fn get_client_balance_returns_200_with_normalized_profile_and_balance() {
    let app = common::running_app().await;
    register_ana(&app).await;

    let answer = get_client_balance(&app, &format!("?user_id={ANA_ID}")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(
        answer.body_text,
        r#"{"client_id":1,"client_name":"Ana Pérez","birth_date":"1990-05-17","document_number":"12345678","country":"AR","balance":0}"#
    );
}

#[actix_web::test]
async fn get_client_balance_returns_negative_balance_as_json_number() {
    let app = common::running_app().await;
    register_ana(&app).await;
    common::post_json_expecting_success(
        &app,
        "/new_credit_transaction",
        &format!(r#"{{"client_id":{ANA_ID},"credit_amount":100.50}}"#),
    )
    .await;
    common::post_json_expecting_success(
        &app,
        "/new_debit_transaction",
        &format!(r#"{{"client_id":{ANA_ID},"debit_amount":30}}"#),
    )
    .await;
    common::post_json_expecting_success(
        &app,
        "/new_debit_transaction",
        &format!(r#"{{"client_id":{ANA_ID},"debit_amount":1000}}"#),
    )
    .await;

    let answer = get_client_balance(&app, &format!("?user_id={ANA_ID}")).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert!(answer.body_text.ends_with(r#""balance":-929.50}"#));
}

#[actix_web::test]
async fn get_client_balance_unknown_id_returns_404_client_not_found() {
    let app = common::running_app().await;

    let answer = get_client_balance(&app, "?user_id=42").await;

    assert_eq!(answer.status, StatusCode::NOT_FOUND);
    assert_eq!(answer.body()["code"], "CLIENT_NOT_FOUND");
}

#[actix_web::test]
async fn get_client_balance_without_user_id_returns_400_invalid_request_json() {
    assert_invalid_request("").await;
}

#[actix_web::test]
async fn get_client_balance_with_user_id_abc_returns_400_invalid_request_json() {
    assert_invalid_request("?user_id=abc").await;
}

#[actix_web::test]
async fn get_client_balance_with_negative_user_id_returns_400_invalid_request_json() {
    assert_invalid_request("?user_id=-1").await;
}

#[actix_web::test]
async fn get_client_balance_with_fractional_user_id_returns_400_invalid_request_json() {
    assert_invalid_request("?user_id=1.5").await;
}
