mod config;
mod session;
mod views;

use fluffer::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::default()
        .address("0.0.0.0:1965")
        .route("/", crate::views::feed)
        .run()
        .await?;

    Ok(())
}
