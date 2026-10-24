//! Entry point: reads settings, starts logging and serves HTTP until a stop signal.

use actix_web::{middleware::Logger, web, App, HttpServer};
use prex_processor::{
    application::ClientService,
    config::Settings,
    http,
    infrastructure::{next_file_counter_in_dir, FsCutOffFileWriter},
};
use tracing_subscriber::EnvFilter;

/// Time in-flight requests get to finish after a stop signal.
const SHUTDOWN_TIMEOUT_SECONDS: u64 = 10;
/// Log level used when `RUST_LOG` is not set.
const DEFAULT_LOG_FILTER: &str = "info";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER)),
        )
        .init();

    let settings = Settings::from_env().unwrap_or_else(|error| {
        eprintln!("invalid configuration: {error}");
        std::process::exit(1);
    });

    tracing::info!(host = %settings.host, port = settings.port, "starting server");

    std::fs::create_dir_all(&settings.cut_off_dir)?;
    let next_file_counter = next_file_counter_in_dir(&settings.cut_off_dir)?;
    tracing::info!(next_file_counter, dir = ?settings.cut_off_dir, "cut-off folder ready");

    let client_service = web::Data::new(ClientService::new(
        FsCutOffFileWriter::new(settings.cut_off_dir.clone()),
        next_file_counter,
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(client_service.clone())
            .wrap(Logger::default())
            .configure(http::configure::<FsCutOffFileWriter>)
    })
    .bind((settings.host.as_str(), settings.port))?
    .shutdown_timeout(SHUTDOWN_TIMEOUT_SECONDS)
    .run()
    .await
}
