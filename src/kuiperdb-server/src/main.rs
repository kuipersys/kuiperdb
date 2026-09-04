use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::Result;
use kuiperdb_core::Database;
use kuiperdb_server::api;

mod telemetry;

#[actix_web::main]
async fn main() -> Result<()> {
    let _guard = telemetry::init_telemetry()?;
    let database_path =
        std::env::var("KUIPERDB_PATH").unwrap_or_else(|_| "./data/kuiper.db".into());
    let bind_address = std::env::var("KUIPERDB_BIND").unwrap_or_else(|_| "0.0.0.0:17001".into());
    let state = web::Data::new(api::AppState {
        database: Database::open(&database_path).await?,
    });
    tracing::info!(path = %database_path, address = %bind_address, "KuiperDB server starting");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Cors::permissive())
            .wrap(Logger::default())
            .configure(api::configure)
    })
    .bind(&bind_address)?
    .run()
    .await?;
    telemetry::shutdown_telemetry();
    Ok(())
}
