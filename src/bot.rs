use std::env;
use chrono::Timelike;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use teloxide::types::{ChatKind, InlineKeyboardButton, InlineKeyboardMarkup, MediaKind, MessageKind};
use serde::Serialize;
use reqwest::Client;

use crate::auth;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize)]
struct EmbyRegisterPayload {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "CopyFromUserId")]
    copy_from_user_id: String,
    #[serde(rename = "UserCopyOptions")]
    user_copy_options: Vec<String>,
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    WaitingRegistrationUsername,
    WaitingRequestDatasource,
    WaitingRequestMediaType {
        data_source: String,
    },
    WaitingRequestMediaID {
        data_source: String,
        media_type: String,
    },
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text.
    Help,
    /// NOOOOOO Check In,
    CheckIn,
    /// Start the purchase procedure.
    Start,
    /// Check the Chat ID,
    ChatID,
    /// Register a new user.
    Register,
    /// Request a new media,
    Request,
    /// Cancel the operation.
    Cancel,
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::CheckIn].endpoint(check_in))
        .branch(case![Command::ChatID].endpoint(chat_id))
        .branch(case![Command::Register].endpoint(register_start))
        .branch(case![Command::Request].endpoint(request_start))
        .branch(case![Command::Cancel].endpoint(cancel));
    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::WaitingRegistrationUsername].endpoint(register_username))
        .branch(case![State::WaitingRequestMediaID { data_source, media_type }].endpoint(request_confirmation))
        .branch(dptree::endpoint(invalid_state));
    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::WaitingRequestDatasource].endpoint(request_media_type))
        .branch(case![State::WaitingRequestMediaType { data_source }].endpoint(request_media_id));
    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler).branch(callback_query_handler)
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    match msg.chat.kind {
        ChatKind::Public(_) => {
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
        }
        _ => {
            bot.send_message(msg.chat.id, "可用命令：\n/help - 显示此帮助\n/register - 注册新用户").await?;
        }
    }
    Ok(())
}

async fn check_in(bot: Bot, msg: Message) -> HandlerResult {
    match msg.chat.kind {
        ChatKind::Public(_) => {
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
                            let toronto_time = chrono::Utc::now().with_timezone(&chrono::FixedOffset::west(5*3600));
                            println!("Toronto Time: {}", toronto_time);
                            if toronto_time.hour() < 16 {
                                let reply = bot.send_message(msg.chat.id, "抱歉，您不是我们的常旅客会员，我们无法在当地时间16:00之前为您办理登记入住。").await?;
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                bot.delete_message(msg.chat.id, msg.id).await?;
                                bot.delete_message(msg.chat.id, reply.id).await?;
                                return Ok(());
                            } else {
                                let reply = bot.send_message(msg.chat.id, "抱歉，本酒店今日房满。").await?;
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                bot.delete_message(msg.chat.id, msg.id).await?;
                                bot.delete_message(msg.chat.id, reply.id).await?;
                            }
                            return Ok(());
                        } else {
                            bot.delete_message(msg.chat.id, msg.id).await?;
                        }
                    }
                }
                _ => {
                    bot.send_message(msg.chat.id, "未知错误，请联系管理员。[Command::CheckIn]").await?;
                }
            }
        }
        _ => {
            let reply = bot.send_message(msg.chat.id, "本站无需每日签到。").await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(msg.chat.id, reply.id).await?;
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
                match submit_emby_register(text.text.clone()).await {
                    Ok(_) => {
                        auth::register(msg.chat.id.0, text.text);
                        bot.send_message(msg.chat.id, "注册成功。").await?;
                    },
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("注册失败。\n{}\n请重新使用 /register 开始注册流程。", e)).await?;
                    }
                }
                dialogue.exit().await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "无效的用户名，请重新输入。").await?;
            }
        }
    }
    Ok(())
}

async fn request_start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.chat.kind {
        ChatKind::Public(_) => {
            let reply = bot.send_message(msg.chat.id, "请在私聊中使用此命令。").await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            bot.delete_message(msg.chat.id, msg.id).await?;
            bot.delete_message(msg.chat.id, reply.id).await?;
        }
        _ => {
            let data_sources = ["TMDB", "BGM.TV"]
                .map(|product| InlineKeyboardButton::callback(product, product));
            bot.send_message(msg.chat.id, "请选择您的数据来源")
                .reply_markup(InlineKeyboardMarkup::new([data_sources]))
                .await?;
            dialogue.update(State::WaitingRequestDatasource).await?;
        }
    }
    Ok(())
}

async fn request_media_type(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(media_source) = &q.data {
        match media_source.as_str() {
            "TMDB" | "BGM.TV" => {
                let media_types = ["电影", "电视剧"]
                    .map(|product| InlineKeyboardButton::callback(product, product));
                bot.send_message(dialogue.chat_id(), "请选择您要请求的媒体类型")
                    .reply_markup(InlineKeyboardMarkup::new([media_types]))
                    .await?;
                let media_source = media_source.clone();
                dialogue.update(State::WaitingRequestMediaType { data_source: media_source }).await?;
            },
            _ => {
                let error_reply = bot.send_message(dialogue.chat_id(), "无效的数据来源，请重新选择。").await?;
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                bot.delete_message(dialogue.chat_id(), error_reply.id).await?;
            }
        }
    }
    Ok(())
}

async fn request_media_id(bot: Bot, dialogue: MyDialogue, data_source: String, q: CallbackQuery) -> HandlerResult {
    if let Some(media_type) = &q.data {
        match media_type.as_str() {
            "电影" | "电视剧" => {
                let media_type = media_type.clone();
                bot.send_message(dialogue.chat_id(), format!("请输入您要从 {} 请求的 {} ID: ", data_source, media_type)).await?;
                dialogue.update(State::WaitingRequestMediaID { data_source, media_type }).await?;
            },
            _ => {
                bot.send_message(dialogue.chat_id(), "无效的媒体类型，请重新选择。").await?;
                return Ok(());
            }
        }
    }
    Ok(())
}

async fn request_confirmation(bot: Bot, dialogue: MyDialogue, msg: Message, data: (String,String)) -> HandlerResult {
    if let MessageKind::Common(common) = msg.kind {
        match common.media_kind {
            MediaKind::Text(text) => {
                if text.text.chars().any(|c| !c.is_digit(10)) {
                    bot.send_message(msg.chat.id, "媒体ID应为纯数字，请重新输入。").await?;
                    return Ok(());
                }
                match data.0.as_str() {
                    "TMDB" => {
                        match data.1.as_str() {
                            "电影" => {
                                bot.send_message(msg.chat.id, format!("https://www.themoviedb.org/{}/{}", "movie", text.text)).await?;
                            },
                            "电视剧" => {
                                bot.send_message(msg.chat.id, format!("https://www.themoviedb.org/{}/{}", "tv", text.text)).await?;
                            },
                            _ => {
                                bot.send_message(msg.chat.id, "未知错误，请联系管理员。[RequestConfirmation]").await?;
                            }
                        }
                    },
                    "BGM.TV" => {
                        bot.send_message(msg.chat.id, format!("https://bgm.tv/subject/{}", text.text)).await?;
                    },
                    _ => {
                        bot.send_message(msg.chat.id, "未知错误，请联系管理员。[RequestConfirmation]").await?;
                    }
                }
                dialogue.exit().await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "无效的ID，ID应为纯数字，请重新输入。").await?;
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

async fn submit_emby_register(username: String) -> Result<(), String> {
    let client = Client::new();
    let emby_url = env::var("EMBY_URL").expect("EMBY_URL must be set");
    let copy_from_user_id = env::var("EMBY_COPY_FROM_USER_ID").expect("EMBY_COPY_FROM_USER_ID must be set");
    let emby_token = env::var("EMBY_TOKEN").expect("EMBY_TOKEN must be set");

    let user = EmbyRegisterPayload {
        name: username,
        copy_from_user_id: copy_from_user_id,
        user_copy_options: vec!["UserPolicy".to_string()],
    };

    let res = client.post(format!("{}/Users/New", emby_url))
        .json(&user)
        .header("X-Emby-Token", emby_token)
        .send()
        .await
        .map_err(|e| format!("请联系管理员。[Error: Emby API]"))?;

    if res.status().is_success() {
        Ok(())
    } else {
        let error_message = res.text().await.map_err(|e| format!("请联系管理员。[Error: res.text]"))?;
        Err(error_message)
    }
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