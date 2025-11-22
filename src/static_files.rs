use actix_web::{web, HttpResponse, Responder, Result, HttpRequest};
use include_dir::{include_dir, Dir};

// 嵌入静态文件目录
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

pub fn configure_static_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/", web::get().to(serve_index))
        .route("/assets/{filename:.*}", web::get().to(serve_asset))
        .default_service(web::route().to(spa_handler));
}

// 服务首页
async fn serve_index() -> Result<impl Responder> {
    serve_embedded_file("index.html").await
}

// 直接服务资产文件（根路径下）
pub async fn serve_asset_direct(path: web::Path<String>) -> Result<impl Responder> {
    let filename = path.into_inner();
    let file_path = format!("assets/{}", filename);
    
    if let Some(file) = STATIC_DIR.get_file(&file_path) {
        let content = file.contents();
        let content_type = get_content_type(&file_path);
        
        Ok(HttpResponse::Ok()
            .content_type(content_type)
            .insert_header(("cache-control", "public, max-age=31536000"))
            .body(content.to_vec()))
    } else {
        Ok(HttpResponse::NotFound().body("File not found"))
    }
}

// 服务资产文件（scope 内）
async fn serve_asset(path: web::Path<String>) -> Result<impl Responder> {
    let filename = path.into_inner();
    let file_path = format!("assets/{}", filename);
    
    if let Some(file) = STATIC_DIR.get_file(&file_path) {
        let content = file.contents();
        let content_type = get_content_type(&file_path);
        
        Ok(HttpResponse::Ok()
            .content_type(content_type)
            .insert_header(("cache-control", "public, max-age=31536000"))
            .body(content.to_vec()))
    } else {
        Ok(HttpResponse::NotFound().body("File not found"))
    }
}

// 获取内容类型
fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

// 服务嵌入的文件
async fn serve_embedded_file(file_path: &str) -> Result<impl Responder> {
    if let Some(file) = STATIC_DIR.get_file(file_path) {
        let content = std::str::from_utf8(file.contents())
            .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid UTF-8"))?;
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(content.to_string()))
    } else {
        // 如果嵌入的文件不存在，返回占位页面
        let placeholder = r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <title>Nyamedia Bot</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background-color: #f5f5f5;
        }
        .container {
            text-align: center;
            padding: 40px;
            background: white;
            border-radius: 12px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.1);
            max-width: 500px;
        }
        h1 { color: #333; margin-bottom: 16px; }
        p { color: #666; margin-bottom: 12px; }
        a { color: #007aff; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Nyamedia Bot</h1>
        <p>前端文件未嵌入到二进制中。请确保编译时静态文件存在。</p>
        <p>API服务正在运行中。</p>
        <p><a href="/requests">查看媒体请求</a></p>
    </div>
</body>
</html>
        "#;
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(placeholder))
    }
}

// SPA fallback - 所有未匹配的路由返回index.html
async fn spa_handler(_req: HttpRequest) -> Result<impl Responder> {
    serve_embedded_file("index.html").await
}