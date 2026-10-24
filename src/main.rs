use actix_web::{middleware::Logger, App, HttpServer};
use prex_processor::{config::Settings, http};
use tracing_subscriber::EnvFilter;

const SHUTDOWN_TIMEOUT_SECONDS: u64 = 10;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env().unwrap_or_else(|error| {
        eprintln!("invalid configuration: {error}");
        std::process::exit(1);
    });

    tracing::info!(host = %settings.host, port = settings.port, "starting server");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .configure(http::configure)
    })
    .bind((settings.host.as_str(), settings.port))?
    .shutdown_timeout(SHUTDOWN_TIMEOUT_SECONDS)
    .run()
    .await
}
