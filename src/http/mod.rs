//! HTTP layer: routes, handlers and request/response shapes.

mod client_balance;
mod error;
mod health;
mod movement;
mod new_client;
mod store_balances;

use actix_web::web::{self, JsonConfig, QueryConfig, ServiceConfig};

use crate::application::CutOffFileWriter;

pub use error::ApiErrorResponse;

/// Registers every route of the service, so `main` and the tests build the same app.
///
/// The caller adds the `web::Data<ClientService<W>>` the handlers read.
pub fn configure<W: CutOffFileWriter + 'static>(service_config: &mut ServiceConfig) {
    service_config
        .app_data(JsonConfig::default().error_handler(error::invalid_json_body))
        .app_data(QueryConfig::default().error_handler(error::invalid_query_string))
        .service(health::health)
        .route(
            "/client_balance",
            web::get().to(client_balance::client_balance::<W>),
        )
        .route("/new_client", web::post().to(new_client::new_client::<W>))
        .route(
            "/new_credit_transaction",
            web::post().to(movement::new_credit_transaction::<W>),
        )
        .route(
            "/new_debit_transaction",
            web::post().to(movement::new_debit_transaction::<W>),
        )
        .route(
            "/store_balances",
            web::post().to(store_balances::store_balances::<W>),
        );
}
