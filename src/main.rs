mod config;
mod session;
mod state;
mod types;
mod views;

use config::Config;
use fluffer::App;
use state::State;
use tracing::info;

static VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("💎 benitoite v{}", VERSION);

    let config = Config::parse()?;
    let state = State::init(&config).await?;

    let app = App::default()
        .address(config.base.bind.clone())
        .path_to_cert(config.base.cert.clone())
        .path_to_key(config.base.key.clone())
        .state(state)
        .route("/", crate::views::feed)
        .route("/p/:id", crate::views::interact)
        .run();

    info!("listening on {}", config.base.bind);

    app.await?;

    Ok(())
}
