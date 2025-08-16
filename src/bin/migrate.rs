use dotenvy::dotenv;
use nyamedia_bot::database;
use std::env;

/// 独立的数据库迁移工具
fn main() {
    dotenv().expect(".env file not found");
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "status" => {
                match database::get_migration_status() {
                    Ok(migrations) => {
                        println!("已应用的迁移:");
                        for migration in migrations {
                            println!("  - {}", migration);
                        }
                    }
                    Err(e) => {
                        eprintln!("获取迁移状态失败: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "run" => {
                match database::run_migrations() {
                    Ok(()) => println!("迁移执行成功"),
                    Err(e) => {
                        eprintln!("迁移执行失败: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "run-with-backup" => {
                match database::run_migrations_with_backup() {
                    Ok(()) => println!("迁移执行成功（已创建备份）"),
                    Err(e) => {
                        eprintln!("迁移执行失败: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "backup" => {
                match database::backup_database() {
                    Ok(backup_path) => println!("数据库备份成功: {}", backup_path),
                    Err(e) => {
                        eprintln!("数据库备份失败: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                print_usage();
                std::process::exit(1);
            }
        }
    } else {
        print_usage();
    }
}

fn print_usage() {
    println!("用法: migrate <command>");
    println!();
    println!("命令:");
    println!("  status           - 显示已应用的迁移状态");
    println!("  run              - 运行待处理的迁移");
    println!("  run-with-backup  - 运行迁移前先备份数据库");
    println!("  backup           - 仅备份数据库");
}