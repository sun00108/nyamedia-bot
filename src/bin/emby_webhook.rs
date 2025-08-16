use dotenvy::dotenv;
use nyamedia_bot::{webhook, database};

// Emby Webhook Server 的 Standalone 实现
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect(".env file not found");
    
    // 初始化日志
    database::init_logger();
    
    // 运行数据库迁移（带备份）
    match database::run_migrations_with_backup() {
        Ok(()) => log::info!("数据库迁移成功"),
        Err(e) => {
            log::error!("数据库迁移失败: {}", e);
            std::process::exit(1);
        }
    }
    
    webhook::run_server().await
}