use diesel::{Connection, SqliteConnection, ConnectionError};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use std::fs;
use std::path::Path;
use chrono::Utc;
use dotenvy::dotenv;
use std::sync::Once;

// 嵌入迁移文件
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

// 确保日志只初始化一次
static INIT_LOGGER: Once = Once::new();

/// 安全的日志初始化函数，避免重复初始化
pub fn init_logger() {
    INIT_LOGGER.call_once(|| {
        pretty_env_logger::init();
    });
}

/// 数据库备份和迁移错误类型
#[derive(Debug)]
pub enum DatabaseError {
    IoError(std::io::Error),
    DieselError(diesel::result::Error),
    MigrationError(Box<dyn std::error::Error + Send + Sync>),
    EnvironmentError(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::IoError(e) => write!(f, "IO错误: {}", e),
            DatabaseError::DieselError(e) => write!(f, "数据库错误: {}", e),
            DatabaseError::MigrationError(e) => write!(f, "迁移错误: {}", e),
            DatabaseError::EnvironmentError(e) => write!(f, "环境变量错误: {}", e),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<std::io::Error> for DatabaseError {
    fn from(err: std::io::Error) -> Self {
        DatabaseError::IoError(err)
    }
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(err: diesel::result::Error) -> Self {
        DatabaseError::DieselError(err)
    }
}

/// 建立数据库连接
pub fn establish_connection() -> Result<SqliteConnection, DatabaseError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| DatabaseError::EnvironmentError("DATABASE_URL must be set".to_string()))?;
    
    SqliteConnection::establish(&database_url)
        .map_err(|e: ConnectionError| DatabaseError::DieselError(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UnableToSendCommand,
            Box::new(e.to_string())
        )))
}

/// 创建数据库备份
pub fn backup_database() -> Result<String, DatabaseError> {
    dotenv().ok();
    
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| DatabaseError::EnvironmentError("DATABASE_URL must be set".to_string()))?;
    
    let database_path = Path::new(&database_url);
    
    // 检查数据库文件是否存在
    if !database_path.exists() {
        log::info!("数据库文件不存在，跳过备份: {}", database_url);
        return Ok("数据库文件不存在，跳过备份".to_string());
    }
    
    // 生成备份文件名（带时间戳）
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("{}.backup.{}.sqlite3", 
        database_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("database"), 
        timestamp
    );
    
    let backup_path = database_path.parent()
        .unwrap_or(Path::new("."))
        .join(backup_filename);
    
    // 执行文件复制
    fs::copy(database_path, &backup_path)?;
    
    let backup_path_str = backup_path.to_string_lossy().to_string();
    log::info!("数据库备份完成: {} -> {}", database_url, backup_path_str);
    
    Ok(backup_path_str)
}

/// 运行数据库迁移（在备份之后）
pub fn run_migrations_with_backup() -> Result<(), DatabaseError> {
    // 1. 先备份数据库
    match backup_database() {
        Ok(backup_path) => {
            log::info!("数据库备份成功: {}", backup_path);
        }
        Err(e) => {
            log::warn!("数据库备份失败: {}", e);
            // 根据需要，可以选择是否继续执行迁移
            // 这里我们选择继续，但会记录警告
        }
    }
    
    // 2. 建立连接并运行迁移
    let mut connection = establish_connection()?;
    
    log::info!("开始运行数据库迁移...");
    
    connection.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DatabaseError::MigrationError(e))?;
    
    log::info!("数据库迁移完成");
    
    Ok(())
}

/// 仅运行迁移（不备份）
pub fn run_migrations() -> Result<(), DatabaseError> {
    let mut connection = establish_connection()?;
    
    log::info!("开始运行数据库迁移...");
    
    connection.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DatabaseError::MigrationError(e))?;
    
    log::info!("数据库迁移完成");
    
    Ok(())
}

/// 获取迁移状态
pub fn get_migration_status() -> Result<Vec<String>, DatabaseError> {
    let mut connection = establish_connection()?;
    
    // 获取已应用的迁移
    let applied_migrations = connection.applied_migrations()
        .map_err(|e| DatabaseError::MigrationError(e))?;
    
    Ok(applied_migrations.into_iter().map(|v| v.to_string()).collect())
}