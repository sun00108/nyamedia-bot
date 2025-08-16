use dotenvy::dotenv;
use nyamedia_bot::{bot, webhook, database};

#[actix_web::main]
async fn main() {
    // Load .env file
    dotenv().expect(".env file not found");
    
    // 初始化日志
    pretty_env_logger::init();

    log::info!("Starting Nyamedia Group Bot...");

    // 运行数据库迁移（带备份）
    match database::run_migrations_with_backup() {
        Ok(()) => log::info!("数据库迁移成功"),
        Err(e) => {
            log::error!("数据库迁移失败: {}", e);
            std::process::exit(1);
        }
    }

    actix_rt::spawn(async move {
        bot::bot_start().await;
    });

    webhook::run_server().await.expect("Failed to start Emby Webhook Server.");
}