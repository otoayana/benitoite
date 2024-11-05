mod config;
mod session;
mod state;
mod types;
mod views;

use std::sync::Arc;

use config::Config;
use fluffer::App;
use state::State;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse()?;
    let state = Arc::new(Mutex::new(State::init(&config).await?));

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
