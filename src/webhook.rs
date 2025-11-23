use std::env;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use teloxide::{prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use diesel::prelude::*;
use crate::models::{MediaRequest, TelegramUser, Media, media_request_status};
use crate::schema::{media_requests, telegram_users, media};
use crate::database;
use crate::static_files;
use crate::scraper;
use chrono::Utc;

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

#[derive(Debug, Serialize, Deserialize)]
struct UpdateRequestPayload {
    request_id: i32,
    new_status: i32,
}

#[derive(Debug, Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
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
                        data.bot.send_message(receipt, format!("新剧集入库: {} ({})\n{} - 第 {} 集 - {}", item.series_name, item.production_year, item.season_name, item.index_number, item.name)).await.ok();
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


fn status_text(status: i32) -> &'static str {
    match status {
        media_request_status::SUBMITTED => "已提交",
        media_request_status::ARCHIVED => "已入库",
        media_request_status::CANCELLED => "已取消",
        media_request_status::INVALID => "不符合规范",
        _ => "未知状态",
    }
}

async fn update_request(
    payload: web::Json<UpdateRequestPayload>,
    data: web::Data<Arc<WebhookData>>
) -> impl Responder {
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: "数据库连接失败".to_string(),
            });
        }
    };

    // 获取请求详情和用户信息
    let request_result: Result<(MediaRequest, TelegramUser), diesel::result::Error> = 
        media_requests::table
            .inner_join(telegram_users::table.on(media_requests::request_user.eq(telegram_users::telegram_id)))
            .select((MediaRequest::as_select(), TelegramUser::as_select()))
            .filter(media_requests::id.eq(payload.request_id))
            .first(&mut conn);

    let (request, user) = match request_result {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::BadRequest().json(ApiResponse {
                success: false,
                message: "请求不存在".to_string(),
            });
        }
    };

    // 检查当前状态是否为已提交
    if request.status != media_request_status::SUBMITTED {
        return HttpResponse::BadRequest().json(ApiResponse {
            success: false,
            message: "只能操作已提交状态的请求".to_string(),
        });
    }

    // 更新请求状态
    let update_result = diesel::update(media_requests::table.filter(media_requests::id.eq(payload.request_id)))
        .set((
            media_requests::status.eq(payload.new_status),
            media_requests::updated_at.eq(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        ))
        .execute(&mut conn);

    if update_result.is_err() {
        return HttpResponse::InternalServerError().json(ApiResponse {
            success: false,
            message: "状态更新失败".to_string(),
        });
    }

    // 发送Telegram通知
    let status_text = match payload.new_status {
        media_request_status::ARCHIVED => "已入库",
        media_request_status::INVALID => "不符合规范",
        media_request_status::CANCELLED => "已取消",
        _ => "未知状态",
    };

    let notification_message = format!(
        "您的媒体请求状态已更新：\n\n📁 来源：{}\n🎬 媒体ID：{}\n📊 状态：{}\n\n{}",
        request.source,
        request.media_id,
        status_text,
        match payload.new_status {
            media_request_status::ARCHIVED => "恭喜！您的请求已成功入库，现在可以在媒体库中找到相关内容。",
            media_request_status::INVALID => "抱歉，您的请求不符合我们的规范要求，请检查后重新提交。",
            media_request_status::CANCELLED => "您的请求已被取消。如有疑问，请联系管理员。",
            _ => "",
        }
    );

    let chat_id = ChatId(user.telegram_id);
    if let Err(_) = data.bot.send_message(chat_id, notification_message).await {
        // 即使通知发送失败，也返回成功，因为状态已经更新
        log::warn!("Failed to send notification to user {}", user.telegram_id);
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("请求状态已更新为：{}", status_text),
    })
}

#[derive(serde::Serialize)]
struct UserCheckResponse {
    registered: bool,
    telegram_username: Option<String>,
    database_username: Option<String>,
    admin: Option<bool>,
}

#[derive(serde::Serialize)]
struct MediaListResponse {
    id: i32,
    title: String,
    poster: Option<String>,
}

#[derive(serde::Serialize)]
struct MediaRequestWithMedia {
    id: i32,
    source: String,
    media_id: String,
    status: i32,
    created_at: String,
    title: Option<String>,
    poster: Option<String>,
}

#[derive(serde::Serialize)]
struct BatchScrapeResult {
    total_processed: usize,
    successful: usize,
    failed: usize,
    errors: Vec<String>,
}

async fn get_pending_requests() -> impl Responder {
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库连接失败"
            }));
        }
    };

    // 查询所有未入库的媒体请求（status = 0）并左连接媒体表
    let pending_requests_result = media_requests::table
        .left_join(media::table.on(media::media_request_id.eq(media_requests::id)))
        .filter(media_requests::status.eq(media_request_status::SUBMITTED))
        .select((
            media_requests::id,
            media_requests::source,
            media_requests::media_id,
            media_requests::status,
            media_requests::created_at,
            media::title.nullable(),
            media::poster.nullable(),
        ))
        .load::<(i32, String, String, i32, String, Option<String>, Option<String>)>(&mut conn);

    match pending_requests_result {
        Ok(requests) => {
            let response: Vec<MediaRequestWithMedia> = requests.into_iter().map(|(id, source, media_id, status, created_at, title, poster)| MediaRequestWithMedia {
                id,
                source,
                media_id,
                status,
                created_at,
                title,
                poster,
            }).collect();
            
            HttpResponse::Ok().json(response)
        },
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库查询失败"
            }))
        }
    }
}

async fn get_archived_requests() -> impl Responder {
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库连接失败"
            }));
        }
    };

    // 查询所有已入库的媒体请求（status = 1）并连接媒体表
    let archived_requests_result = media_requests::table
        .inner_join(media::table.on(media::media_request_id.eq(media_requests::id)))
        .filter(media_requests::status.eq(media_request_status::ARCHIVED))
        .select((
            media_requests::id,
            media_requests::source,
            media_requests::media_id,
            media_requests::status,
            media_requests::created_at,
            media::title,
            media::poster.nullable(),
        ))
        .load::<(i32, String, String, i32, String, String, Option<String>)>(&mut conn);

    match archived_requests_result {
        Ok(requests) => {
            let response: Vec<MediaRequestWithMedia> = requests.into_iter().map(|(id, source, media_id, status, created_at, title, poster)| MediaRequestWithMedia {
                id,
                source,
                media_id,
                status,
                created_at,
                title: Some(title),
                poster,
            }).collect();
            
            HttpResponse::Ok().json(response)
        },
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库查询失败"
            }))
        }
    }
}

async fn batch_scrape_media() -> impl Responder {
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库连接失败"
            }));
        }
    };

    // 查询所有没有对应媒体信息的请求
    let unscraped_requests_result = media_requests::table
        .left_join(media::table.on(media::media_request_id.eq(media_requests::id)))
        .filter(media::id.is_null()) // 没有对应的媒体信息
        .select((
            media_requests::id,
            media_requests::source,
            media_requests::media_id,
        ))
        .load::<(i32, String, String)>(&mut conn);

    let unscraped_requests = match unscraped_requests_result {
        Ok(requests) => requests,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "查询未刮削媒体失败"
            }));
        }
    };

    let total_processed = unscraped_requests.len();
    let mut successful = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    // 批量处理每个请求
    for (request_id, source, media_id) in unscraped_requests {
        // 确定API参数
        let (api_source, api_media_type) = match source.as_str() {
            "TMDB/MV" => ("tmdb", "movie"),
            "TMDB/TV" => ("tmdb", "tv"),
            "BGM.TV" => ("bgm", "subject"),
            _ => {
                let error_msg = format!("请求ID {}: 不支持的媒体源: {}", request_id, source);
                errors.push(error_msg);
                failed += 1;
                continue;
            }
        };

        // 调用刮削API
        match scraper::scrape_media_info(api_source, api_media_type, &media_id).await {
            Ok(media_info) => {
                // 保存媒体信息到数据库
                match scraper::save_media_to_db(&mut conn, request_id, &media_info) {
                    Ok(_) => {
                        successful += 1;
                        log::info!("成功刮削请求ID {}: {} {}", request_id, source, media_id);
                    },
                    Err(e) => {
                        let error_msg = format!("请求ID {}: 保存失败: {:?}", request_id, e);
                        errors.push(error_msg);
                        failed += 1;
                        log::warn!("请求ID {} 保存失败: {:?}", request_id, e);
                    }
                }
            },
            Err(e) => {
                let error_msg = format!("请求ID {}: 刮削失败: {}", request_id, e);
                errors.push(error_msg);
                failed += 1;
                log::warn!("请求ID {} 刮削失败: {}", request_id, e);
            }
        }

        // 添加短暂延迟以避免API限制
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    let result = BatchScrapeResult {
        total_processed,
        successful,
        failed,
        errors,
    };

    log::info!(
        "批量刮削完成: 总计={}, 成功={}, 失败={}", 
        total_processed, successful, failed
    );

    HttpResponse::Ok().json(result)
}

async fn get_media_list() -> impl Responder {
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库连接失败"
            }));
        }
    };

    // 查询所有媒体数据
    let media_result = media::table
        .load::<Media>(&mut conn);

    match media_result {
        Ok(media_list) => {
            let response: Vec<MediaListResponse> = media_list.into_iter().map(|m| MediaListResponse {
                id: m.id,
                title: m.title,
                poster: m.poster,
            }).collect();
            
            HttpResponse::Ok().json(response)
        },
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库查询失败"
            }))
        }
    }
}

async fn check_user_registration(path: web::Path<i64>) -> impl Responder {
    let telegram_id = path.into_inner();
    
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库连接失败"
            }));
        }
    };

    // 查询用户是否存在
    let user_result = telegram_users::table
        .filter(telegram_users::telegram_id.eq(telegram_id))
        .first::<TelegramUser>(&mut conn);

    match user_result {
        Ok(user) => {
            HttpResponse::Ok().json(UserCheckResponse {
                registered: true,
                telegram_username: None, // 这个需要从 Telegram 数据获取
                database_username: Some(user.username),
                admin: Some(user.admin),
            })
        },
        Err(diesel::result::Error::NotFound) => {
            HttpResponse::Ok().json(UserCheckResponse {
                registered: false,
                telegram_username: None,
                database_username: None,
                admin: None,
            })
        },
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "数据库查询失败"
            }))
        }
    }
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

    let bind_address = env::var("WEBHOOK_BIND_ADDRESS").unwrap();
    let bind_port = env::var("WEBHOOK_BIND_PORT").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone()))
            .service(web::resource("/webhook").route(web::post().to(handle_webhook)))
            .service(web::resource("/api/check_user/{telegram_id}").route(web::get().to(check_user_registration)))
            .service(web::resource("/api/media").route(web::get().to(get_media_list)))
            .service(web::resource("/api/pending").route(web::get().to(get_pending_requests)))
            .service(web::resource("/api/archived").route(web::get().to(get_archived_requests)))
            .service(web::resource("/api/update-request").route(web::post().to(update_request)))
            .service(web::resource("/api/batch-scrape").route(web::post().to(batch_scrape_media)))
            .service(web::resource("/assets/{filename:.*}").route(web::get().to(static_files::serve_asset_direct)))
            .configure(static_files::configure_static_routes)
    })
        .bind(format!("{}:{}",bind_address,bind_port))?
        .run()
        .await
}