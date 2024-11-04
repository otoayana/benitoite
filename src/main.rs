mod config;
mod session;
mod types;
mod views;

use fluffer::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::default()
        .address("0.0.0.0:1965")
        .route("/", crate::views::feed)
        .route("/p/:did/:type/:id", crate::views::interact)
        .run()
        .await?;

    Ok(())
}
