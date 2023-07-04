extern crate argonautica;

use backend::{
    startup::Application,
    util::{configuration::get_config, telemetry},
};
use color_eyre::Result;

#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();
    let config = get_config()?;

    let subscriber = telemetry::get_subscriber(config.environment.clone());
    telemetry::init_subscriber(subscriber)?;

    let application = Application::build(config.clone(), None).await?;

    tracing::info!(target: "backend", "Listening on {}://{}:{}", config.server.scheme, config.server.port, config.server.host);

    application.run_until_stopped().await?;

    Ok(())
}
