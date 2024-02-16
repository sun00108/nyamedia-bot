use dotenvy::dotenv;
use nyamedia_bot::webhook;

// Emby Webhook Server 的 Standalone 实现
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect(".env file not found");
    webhook::run_server().await
}