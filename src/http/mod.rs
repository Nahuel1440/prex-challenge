//! HTTP layer: routes, handlers and request/response shapes.

mod client_balance;
mod error;
mod health;
mod movement;
mod new_client;

use actix_web::web::{JsonConfig, QueryConfig, ServiceConfig};

pub use error::ApiErrorResponse;

/// Registers every route of the service, so `main` and the tests build the same app.
///
/// The caller adds the `web::Data<ClientService>` the handlers read.
pub fn configure(service_config: &mut ServiceConfig) {
    service_config
        .app_data(JsonConfig::default().error_handler(error::invalid_json_body))
        .app_data(QueryConfig::default().error_handler(error::invalid_query_string))
        .service(health::health)
        .service(client_balance::client_balance)
        .service(new_client::new_client)
        .service(movement::new_credit_transaction)
        .service(movement::new_debit_transaction);
}
