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
use crate::cli_auth;
use crate::onedrive;
use crate::media_upload;
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

#[derive(Debug, Deserialize)]
struct CreateMediaUploadRequestPayload {
    request_id: i32,
    season: Option<i32>,
    episode: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct CreateMediaUploadSessionPayload {
    request_code: String,
    file_name: String,
    file_size: u64,
    conflict_behavior: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompleteMediaUploadPayload {
    request_code: String,
    file_name: String,
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

    // 获取请求详情
    let request = match media_requests::table
        .filter(media_requests::id.eq(payload.request_id))
        .first::<MediaRequest>(&mut conn)
    {
        Ok(request) => request,
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

    let chat_id = ChatId(request.request_user);
    if let Err(_) = data.bot.send_message(chat_id, notification_message).await {
        // 即使通知发送失败，也返回成功，因为状态已经更新
        log::warn!("Failed to send notification to user {}", request.request_user);
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

async fn create_media_upload_request(
    req: actix_web::HttpRequest,
    payload: web::Json<CreateMediaUploadRequestPayload>,
) -> impl Responder {
    let claims = match crate::cli_auth::middleware::verify_bearer_token(&req, &["upload:create"]) {
        Ok(claims) => claims,
        Err(err) => {
            return match err {
                crate::cli_auth::service::ServiceError::Unauthorized(message) => {
                    HttpResponse::Unauthorized().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::BadRequest(message) => {
                    HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::InvalidClientId => {
                    HttpResponse::BadRequest().json(serde_json::json!({ "error": "client_id 不被允许" }))
                }
                crate::cli_auth::service::ServiceError::Conflict(message) => {
                    HttpResponse::Conflict().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::Config(message)
                | crate::cli_auth::service::ServiceError::Internal(message) => {
                    HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }))
                }
            };
        }
    };

    match media_upload::create_upload_request(
        claims.telegram_user_id,
        media_upload::CreateMediaUploadRequestInput {
            request_id: payload.request_id,
            season: payload.season,
            episode: payload.episode,
        },
    ) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(media_upload::MediaUploadError::BadRequest(message)) => {
            HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
        }
        Err(media_upload::MediaUploadError::Conflict(message)) => {
            HttpResponse::Conflict().json(serde_json::json!({ "error": message }))
        }
        Err(media_upload::MediaUploadError::Internal(message)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }))
        }
    }
}

async fn create_media_upload_session(
    onedrive_service: web::Data<onedrive::service::OnedriveService>,
    req: actix_web::HttpRequest,
    payload: web::Json<CreateMediaUploadSessionPayload>,
) -> impl Responder {
    match crate::cli_auth::middleware::verify_bearer_token(&req, &["upload:create"]) {
        Ok(_) => {}
        Err(err) => {
            return match err {
                crate::cli_auth::service::ServiceError::Unauthorized(message) => {
                    HttpResponse::Unauthorized().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::BadRequest(message) => {
                    HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::InvalidClientId => {
                    HttpResponse::BadRequest().json(serde_json::json!({ "error": "client_id 不被允许" }))
                }
                crate::cli_auth::service::ServiceError::Conflict(message) => {
                    HttpResponse::Conflict().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::Config(message)
                | crate::cli_auth::service::ServiceError::Internal(message) => {
                    HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }))
                }
            };
        }
    }

    let (upload_request, session_input) = match media_upload::get_upload_request_for_session(
        media_upload::CreateMediaUploadSessionInput {
            request_code: payload.request_code.clone(),
            file_name: payload.file_name.clone(),
            file_size: payload.file_size,
            conflict_behavior: payload.conflict_behavior.clone(),
        },
    ) {
        Ok(result) => result,
        Err(media_upload::MediaUploadError::BadRequest(message)) => {
            return HttpResponse::BadRequest().json(serde_json::json!({ "error": message }));
        }
        Err(media_upload::MediaUploadError::Conflict(message)) => {
            return HttpResponse::Conflict().json(serde_json::json!({ "error": message }));
        }
        Err(media_upload::MediaUploadError::Internal(message)) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }));
        }
    };

    let target_path = format!("{}{}", upload_request.target_path, session_input.file_name);
    let conflict_behavior = session_input
        .conflict_behavior
        .as_deref()
        .unwrap_or("replace");

    match onedrive_service
        .create_upload_session(
            &target_path,
            Some(&session_input.file_name),
            session_input.file_size,
            conflict_behavior,
        )
        .await
    {
        Ok(result) => HttpResponse::Ok().json(media_upload::CreateMediaUploadSessionResult {
            upload_url: result.upload_url,
            expiration_date_time: result.expiration_date_time,
            path: target_path,
        }),
        Err(onedrive::service::OnedriveError::BadRequest(message)) => {
            HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
        }
        Err(onedrive::service::OnedriveError::Forbidden(message)) => {
            HttpResponse::Forbidden().json(serde_json::json!({ "error": message }))
        }
        Err(onedrive::service::OnedriveError::Unauthorized(message)) => {
            HttpResponse::Unauthorized().json(serde_json::json!({ "error": message }))
        }
        Err(onedrive::service::OnedriveError::Conflict(message)) => {
            HttpResponse::Conflict().json(serde_json::json!({ "error": message }))
        }
        Err(onedrive::service::OnedriveError::Upstream(message)) => {
            HttpResponse::BadGateway().json(serde_json::json!({ "error": message }))
        }
        Err(onedrive::service::OnedriveError::Config(message))
        | Err(onedrive::service::OnedriveError::Internal(message)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }))
        }
    }
}

async fn complete_media_upload(
    req: actix_web::HttpRequest,
    payload: web::Json<CompleteMediaUploadPayload>,
) -> impl Responder {
    match crate::cli_auth::middleware::verify_bearer_token(&req, &["upload:create"]) {
        Ok(_) => {}
        Err(err) => {
            return match err {
                crate::cli_auth::service::ServiceError::Unauthorized(message) => {
                    HttpResponse::Unauthorized().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::BadRequest(message) => {
                    HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::InvalidClientId => {
                    HttpResponse::BadRequest().json(serde_json::json!({ "error": "client_id 不被允许" }))
                }
                crate::cli_auth::service::ServiceError::Conflict(message) => {
                    HttpResponse::Conflict().json(serde_json::json!({ "error": message }))
                }
                crate::cli_auth::service::ServiceError::Config(message)
                | crate::cli_auth::service::ServiceError::Internal(message) => {
                    HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }))
                }
            };
        }
    }

    match media_upload::complete_upload_request(media_upload::CompleteMediaUploadInput {
        request_code: payload.request_code.clone(),
        file_name: payload.file_name.clone(),
    }) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(media_upload::MediaUploadError::BadRequest(message)) => {
            HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
        }
        Err(media_upload::MediaUploadError::Conflict(message)) => {
            HttpResponse::Conflict().json(serde_json::json!({ "error": message }))
        }
        Err(media_upload::MediaUploadError::Internal(message)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": message }))
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
    let onedrive_service = onedrive::service::OnedriveService::from_env()
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", err)))?;

    let bind_address = env::var("WEBHOOK_BIND_ADDRESS").unwrap();
    let bind_port = env::var("WEBHOOK_BIND_PORT").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone()))
            .app_data(web::Data::new(onedrive_service.clone()))
            .service(web::resource("/webhook").route(web::post().to(handle_webhook)))
            .service(web::resource("/api/check_user/{telegram_id}").route(web::get().to(check_user_registration)))
            .service(web::resource("/api/media/upload-requests").route(web::post().to(create_media_upload_request)))
            .service(web::resource("/api/media/upload-sessions").route(web::post().to(create_media_upload_session)))
            .service(web::resource("/api/media/upload-completions").route(web::post().to(complete_media_upload)))
            .service(web::resource("/api/media").route(web::get().to(get_media_list)))
            .service(web::resource("/api/pending").route(web::get().to(get_pending_requests)))
            .service(web::resource("/api/archived").route(web::get().to(get_archived_requests)))
            .service(web::resource("/api/update-request").route(web::post().to(update_request)))
            .service(web::resource("/api/batch-scrape").route(web::post().to(batch_scrape_media)))
            .service(web::resource("/assets/{filename:.*}").route(web::get().to(static_files::serve_asset_direct)))
            .configure(cli_auth::http::configure)
            .configure(onedrive::http::configure)
            .configure(static_files::configure_static_routes)
    })
        .bind(format!("{}:{}",bind_address,bind_port))?
        .run()
        .await
}
