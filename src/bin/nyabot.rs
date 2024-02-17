use dotenvy::dotenv;
use nyamedia_bot::{bot, webhook};

#[actix_web::main]
async fn main() {
    // Load .env file
    dotenv().expect(".env file not found");

    log::info!("Starting Nyamedia Group Bot...");

    actix_rt::spawn(async move {
        bot::bot_start().await;
    });

    webhook::run_server().await.expect("Failed to start Emby Webhook Server.");
}