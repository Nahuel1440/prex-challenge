//! HTTP layer: routes, handlers and request/response shapes.

mod error;
mod health;
mod new_client;

use actix_web::web::{JsonConfig, ServiceConfig};

pub use error::ApiErrorResponse;

/// Registers every route of the service, so `main` and the tests build the same app.
///
/// The caller adds the `web::Data<ClientService>` the handlers read.
pub fn configure(service_config: &mut ServiceConfig) {
    service_config
        .app_data(JsonConfig::default().error_handler(error::invalid_json_body))
        .service(health::health)
        .service(new_client::new_client);
}
