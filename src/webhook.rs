use std::env;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use teloxide::{prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

struct WebhookData {
    bot: Bot,
    chat_list: Vec<i64>,
    series_ids: Mutex<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebhookPayload {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Event")]
    event: String,
    #[serde(rename = "Item")]
    item: Option<Item>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "IndexNumber")]
    index_number: u16,
    #[serde(rename = "ProductionYear")]
    production_year: u16,
    #[serde(rename = "SeriesName")]
    series_name: String,
    #[serde(rename = "SeriesId")]
    series_id: String,
    #[serde(rename = "SeasonName")]
    season_name: String,
}

async fn handle_webhook(payload: web::Json<WebhookPayload>, data: web::Data<Arc<WebhookData>>) -> impl Responder {
    match payload.event.as_str() {
        "library.new" => {
            if let Some(item) = &payload.item {
                let mut series_ids = data.series_ids.lock().unwrap();
                if !series_ids.contains(&item.series_id) {
                    series_ids.push(item.series_id.clone());
                    for chat_id in &data.chat_list { // <- 之后更改为从数据库中读取
                        let receipt = ChatId(*chat_id);
                        data.bot.send_message(receipt, format!("新剧集入库: {} ({})\n{} - 第{}集 - {}", item.series_name, item.production_year, item.season_name, item.index_number, item.name)).await.ok();
                    }
                }
            } else {
                // 之后实现
            }
        },
        _ => {
            // 之后实现
        }
    }
    HttpResponse::Ok()
}

pub async fn run_server() -> std::io::Result<()> {
    println!("Starting Emby Webhook Server...");

    let chat_list = env::var("WEBHOOK_NOTIFY_CHAT");
    let chat_list = match chat_list {
        Ok(chat_list) => {
            chat_list.split(",").map(|x| x.parse::<i64>().unwrap()).collect::<Vec<i64>>()
        },
        Err(_) => {
            Vec::new()
        }
    };

    let data = Arc::new(WebhookData {
        bot: Bot::from_env(),
        chat_list,
        series_ids: Mutex::new(Vec::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone()))
            .service(web::resource("/webhook").route(web::post().to(handle_webhook)))
    })
        .bind("127.0.0.1:3000")?
        .run()
        .await
}