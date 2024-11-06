mod config;
mod session;
mod state;
mod types;
mod views;

use config::Config;
use fluffer::App;
use state::State;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse()?;
    let state = State::init(&config).await?;

    App::default()
        .address(config.base.bind)
        .path_to_cert(config.base.cert.clone())
        .path_to_key(config.base.key.clone())
        .state(state)
        .route("/", crate::views::feed)
        .route("/p/:id", crate::views::interact)
        .run()
        .await?;

    Ok(())
}
