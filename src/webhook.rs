use std::env;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use teloxide::{prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use diesel::prelude::*;
use crate::models::{MediaRequest, TelegramUser, media_request_status};
use crate::schema::{media_requests, telegram_users};
use crate::database;
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

async fn get_media_requests() -> impl Responder {
    let mut conn = match database::establish_connection() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("数据库连接失败"),
    };

    let requests_result: Result<Vec<(MediaRequest, TelegramUser)>, diesel::result::Error> = 
        media_requests::table
            .inner_join(telegram_users::table.on(media_requests::request_user.eq(telegram_users::telegram_id)))
            .select((MediaRequest::as_select(), TelegramUser::as_select()))
            .order(media_requests::created_at.desc())
            .load(&mut conn);

    let requests = match requests_result {
        Ok(requests) => requests,
        Err(_) => return HttpResponse::InternalServerError().body("查询失败"),
    };

    let status_text = |status: i32| match status {
        media_request_status::SUBMITTED => "已提交",
        media_request_status::ARCHIVED => "已入库", 
        media_request_status::CANCELLED => "被取消",
        media_request_status::INVALID => "不符合规范",
        _ => "未知状态",
    };

    let html = format!(r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>媒体请求列表</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            overflow: hidden;
        }}
        .header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 30px;
            text-align: center;
        }}
        .header h1 {{
            margin: 0;
            font-size: 2.5em;
            font-weight: 300;
        }}
        .stats {{
            display: flex;
            justify-content: space-around;
            padding: 20px;
            background: #f8f9fa;
            border-bottom: 1px solid #e9ecef;
        }}
        .stat {{
            text-align: center;
        }}
        .stat-number {{
            font-size: 2em;
            font-weight: bold;
            color: #495057;
        }}
        .stat-label {{
            color: #6c757d;
            font-size: 0.9em;
        }}
        .table-container {{
            overflow-x: auto;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
        }}
        th, td {{
            padding: 15px;
            text-align: left;
            border-bottom: 1px solid #e9ecef;
        }}
        th {{
            background-color: #f8f9fa;
            font-weight: 600;
            color: #495057;
            position: sticky;
            top: 0;
        }}
        tr:hover {{
            background-color: #f8f9fa;
        }}
        .status {{
            padding: 6px 12px;
            border-radius: 20px;
            font-size: 0.85em;
            font-weight: 500;
        }}
        .status-submitted {{
            background-color: #fff3cd;
            color: #856404;
        }}
        .status-archived {{
            background-color: #d4edda;
            color: #155724;
        }}
        .status-cancelled {{
            background-color: #f8d7da;
            color: #721c24;
        }}
        .status-invalid {{
            background-color: #f1f3f4;
            color: #5f6368;
        }}
        .no-data {{
            text-align: center;
            padding: 50px;
            color: #6c757d;
        }}
        .media-id {{
            font-family: 'Monaco', 'Menlo', monospace;
            background-color: #f8f9fa;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 0.9em;
        }}
        .source {{
            font-weight: 500;
            color: #495057;
        }}
        .actions {{
            display: flex;
            gap: 8px;
        }}
        .btn {{
            padding: 6px 12px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 0.85em;
            font-weight: 500;
            transition: all 0.2s;
        }}
        .btn:hover {{
            transform: translateY(-1px);
            box-shadow: 0 2px 4px rgba(0,0,0,0.2);
        }}
        .btn:disabled {{
            opacity: 0.5;
            cursor: not-allowed;
            transform: none;
            box-shadow: none;
        }}
        .btn-archive {{
            background-color: #28a745;
            color: white;
        }}
        .btn-archive:hover:not(:disabled) {{
            background-color: #218838;
        }}
        .btn-invalid {{
            background-color: #6c757d;
            color: white;
        }}
        .btn-invalid:hover:not(:disabled) {{
            background-color: #5a6268;
        }}
        .btn-cancel {{
            background-color: #dc3545;
            color: white;
        }}
        .btn-cancel:hover:not(:disabled) {{
            background-color: #c82333;
        }}
        .toast {{
            position: fixed;
            bottom: 20px;
            right: 20px;
            background: #333;
            color: white;
            padding: 12px 20px;
            border-radius: 4px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            transform: translateX(400px);
            transition: transform 0.3s ease;
            z-index: 1000;
        }}
        .toast.show {{
            transform: translateX(0);
        }}
        .toast.success {{
            background: #28a745;
        }}
        .toast.error {{
            background: #dc3545;
        }}
        .loading {{
            display: inline-block;
            width: 12px;
            height: 12px;
            border: 2px solid #ffffff40;
            border-top: 2px solid #ffffff;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin-right: 8px;
        }}
        @keyframes spin {{
            0% {{ transform: rotate(0deg); }}
            100% {{ transform: rotate(360deg); }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>媒体请求列表</h1>
        </div>
        <div class="stats">
            <div class="stat">
                <div class="stat-number">{}</div>
                <div class="stat-label">总请求数</div>
            </div>
            <div class="stat">
                <div class="stat-number">{}</div>
                <div class="stat-label">已提交</div>
            </div>
            <div class="stat">
                <div class="stat-number">{}</div>
                <div class="stat-label">已入库</div>
            </div>
            <div class="stat">
                <div class="stat-number">{}</div>
                <div class="stat-label">其他状态</div>
            </div>
        </div>
        <div class="table-container">
"#,
        requests.len(),
        requests.iter().filter(|(req, _)| req.status == media_request_status::SUBMITTED).count(),
        requests.iter().filter(|(req, _)| req.status == media_request_status::ARCHIVED).count(),
        requests.iter().filter(|(req, _)| req.status != media_request_status::SUBMITTED && req.status != media_request_status::ARCHIVED).count()
    );

    let mut table_html = html;

    if requests.is_empty() {
        table_html.push_str(r#"
            <div class="no-data">
                <h3>暂无媒体请求</h3>
                <p>还没有任何媒体请求记录。</p>
            </div>
        "#);
    } else {
        table_html.push_str(r#"
            <table>
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>来源</th>
                        <th>媒体ID</th>
                        <th>请求用户</th>
                        <th>状态</th>
                        <th>创建时间</th>
                        <th>更新时间</th>
                        <th>操作</th>
                    </tr>
                </thead>
                <tbody>
        "#);

        for (request, user) in requests {
            let status_class = match request.status {
                media_request_status::SUBMITTED => "status-submitted",
                media_request_status::ARCHIVED => "status-archived",
                media_request_status::CANCELLED => "status-cancelled",
                media_request_status::INVALID => "status-invalid",
                _ => "status-invalid",
            };

            let actions_html = if request.status == media_request_status::SUBMITTED {
                format!(r#"
                    <div class="actions">
                        <button class="btn btn-archive" onclick="updateStatus({}, {}, '入库')">入库</button>
                        <button class="btn btn-invalid" onclick="updateStatus({}, {}, '不合规')">不合规</button>
                        <button class="btn btn-cancel" onclick="updateStatus({}, {}, '取消')">取消</button>
                    </div>
                "#, request.id, media_request_status::ARCHIVED, request.id, media_request_status::INVALID, request.id, media_request_status::CANCELLED)
            } else {
                "-".to_string()
            };

            table_html.push_str(&format!(r#"
                    <tr>
                        <td>{}</td>
                        <td><span class="source">{}</span></td>
                        <td><code class="media-id">{}</code></td>
                        <td>{}</td>
                        <td><span class="status {}">{}</span></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>
            "#,
                request.id,
                request.source,
                request.media_id,
                user.username,
                status_class,
                status_text(request.status),
                request.created_at,
                request.updated_at,
                actions_html
            ));
        }

        table_html.push_str(r#"
                </tbody>
            </table>
        "#);
    }

    table_html.push_str(r#"
        </div>
    </div>
    <div id="toast"></div>
    <script>
        function showToast(message, type = 'success') {
            const toast = document.getElementById('toast');
            toast.textContent = message;
            toast.className = `toast ${type} show`;
            
            setTimeout(() => {
                toast.className = toast.className.replace('show', '');
            }, 3000);
        }

        function updateStatus(requestId, newStatus, actionName) {
            if (!confirm(`确定要将此请求标记为"${actionName}"吗？`)) {
                return;
            }

            const buttons = document.querySelectorAll(`[onclick*="${requestId}"]`);
            buttons.forEach(btn => {
                btn.disabled = true;
                btn.innerHTML = '<span class="loading"></span>' + btn.textContent;
            });

            fetch('/update_request', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    request_id: requestId,
                    new_status: newStatus
                })
            })
            .then(response => response.json())
            .then(data => {
                if (data.success) {
                    showToast(data.message, 'success');
                    setTimeout(() => {
                        window.location.reload();
                    }, 1500);
                } else {
                    showToast(data.message, 'error');
                    buttons.forEach(btn => {
                        btn.disabled = false;
                        btn.innerHTML = btn.textContent.replace(/.*/, actionName);
                    });
                }
            })
            .catch(error => {
                showToast('操作失败，请重试', 'error');
                buttons.forEach(btn => {
                    btn.disabled = false;
                    btn.innerHTML = btn.textContent.replace(/.*/, actionName);
                });
            });
        }
    </script>
</body>
</html>
    "#);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(table_html)
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
            .service(web::resource("/requests").route(web::get().to(get_media_requests)))
            .service(web::resource("/update_request").route(web::post().to(update_request)))
    })
        .bind(format!("{}:{}",bind_address,bind_port))?
        .run()
        .await
}