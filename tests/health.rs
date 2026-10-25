use actix_web::{http::StatusCode, test, App};
use prex_processor::{http, infrastructure::FsCutOffFileWriter};
use serde_json::{json, Value};

#[actix_web::test]
async fn health_returns_200_with_status_ok() {
    let app = test::init_service(App::new().configure(http::configure::<FsCutOffFileWriter>)).await;

    let response =
        test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body, json!({ "status": "ok" }));
}
