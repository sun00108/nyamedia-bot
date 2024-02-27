use std::env;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use teloxide::types::{ChatKind, MediaKind, MessageKind};

use crate::auth;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    WaitingRegistrationUsername
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text.
    Help,
    /// Start the purchase procedure.
    Start,
    /// Check the Chat ID,
    ChatID,
    /// Register a new user.
    Register,
    /// Cancel the operation.
    Cancel,
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::ChatID].endpoint(chat_id))
        .branch(case![Command::Register].endpoint(register_start))
        .branch(case![Command::Cancel].endpoint(cancel));
    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::WaitingRegistrationUsername].endpoint(register_username))
        .branch(dptree::endpoint(invalid_state));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
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
        }
        _ => {
            bot.send_message(msg.chat.id, "未知错误，请联系管理员。[Command::Help]").await?;
        }
    }
    Ok(())
}

async fn chat_id(bot: Bot, msg: Message) -> HandlerResult {
    match msg.kind {
        MessageKind::Common(common) => { // 这里的 MessageKind::Common 仅仅为了提取 user 验证用户管理员权限。
            if let Some(user) = common.from {
                let is_admin = auth::check_admin(user.id.0 as i64);
                if !is_admin {
                    let reply = bot.send_message(msg.chat.id, "抱歉，您没有权限使用这个命令。").await?;
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    bot.delete_message(msg.chat.id, msg.id).await?;
                    bot.delete_message(msg.chat.id, reply.id).await?;
                    return Ok(());
                }
                bot.send_message(msg.chat.id, format!("Chat ID: {}", msg.chat.id)).await?;
            }
        },
        _ => {
            bot.send_message(msg.chat.id, "未知错误，请联系管理员。[Command::ChatId]").await?;
        }
    }
    Ok(())
}

async fn register_start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.chat.kind {
        ChatKind::Public(_) => {
            let reply = bot.send_message(msg.chat.id, "请在私聊中使用此命令。").await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(msg.chat.id, reply.id).await?;
        }
        _ => {
            if auth::check_registered(msg.chat.id.0) {
                bot.send_message(msg.chat.id, "您已经注册过了。").await?;
            } else {
                bot.send_message(msg.chat.id, "请输入您的用户名：").await?;
                dialogue.update(State::WaitingRegistrationUsername).await?;
            }
        }
    }
    Ok(())
}

async fn register_username(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    if let MessageKind::Common(common) = msg.kind {
        match common.media_kind {
            MediaKind::Text(text) => {
                if text.text.starts_with("/") {
                    bot.send_message(msg.chat.id, "用户名不能以 / 开头，请重新输入。").await?;
                    return Ok(());
                }
                auth::register(msg.chat.id.0, text.text);
                bot.send_message(msg.chat.id, "注册成功！").await?;
                dialogue.update(State::Start).await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "无效的用户名，请重新输入。").await?;
            }
        }
    }
    Ok(())
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "操作已取消。").await?;
    dialogue.exit().await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    match msg.chat.kind {
        ChatKind::Private(_) => {
            bot.send_message(msg.chat.id, "无效的bot命令，请使用 /help 查看可用命令。").await?;
        },
        _ => {
            // 当 bot 拥有 admin 权限的时候就会收到每一条消息。
        }
    }
    Ok(())
}

pub async fn bot_start() {
    pretty_env_logger::init();
    log::info!("Starting bot...");
    let bot = Bot::from_env();
    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}