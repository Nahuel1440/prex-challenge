//! HTTP layer: routes, handlers and request/response shapes.

mod health;

use actix_web::web::ServiceConfig;

/// Registers every route of the service, so `main` and the tests build the same app.
pub fn configure(service_config: &mut ServiceConfig) {
    service_config.service(health::health);
}
