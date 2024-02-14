use std::env;
use dotenvy::dotenv;
use teloxide::{prelude::*, utils::command::BotCommands};
use teloxide::types::MessageKind;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Load .env file
    dotenv().expect(".env file not found");

    log::info!("Starting Nyamedia Group Bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;

}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            match msg.kind {
                MessageKind::Common(common) => {
                    if let Some(user) = common.from {
                        // Anti Spam Module - 防止某些特定的人使用这个命令
                        let disabled_users = env::var("DISABLED_USERS");
                        let disabled_users = match disabled_users {
                            Ok(disabled_users) => {
                                disabled_users.split(",").map(|x| x.parse::<u64>().unwrap()).collect::<Vec<u64>>()
                            },
                            Err(_) => {
                                Vec::new()
                            }
                        };
                        if disabled_users.contains(&user.id.0) {
                            bot.send_message(msg.chat.id, "抱歉，您没有权限使用这个命令。").await?;
                            return Ok(());
                        } else {
                            let sent_msg = bot.send_message(msg.chat.id, "请查看群简介。").await?;
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            bot.delete_message(msg.chat.id, msg.id).await?;
                            bot.delete_message(msg.chat.id, sent_msg.id).await?;
                        }
                    }
                },
                _ => {
                    bot.send_message(msg.chat.id, "未知错误，请联系管理员。").await?;
                }
            }
        }
    };

    Ok(())
}

