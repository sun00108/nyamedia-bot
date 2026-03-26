use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::cli_auth::middleware;

use super::service::{OnedriveError, OnedriveService};

#[derive(Debug, Deserialize)]
struct CreateUploadSessionRequest {
    path: String,
    file_name: Option<String>,
    file_size: u64,
    conflict_behavior: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/onedrive")
            .route("/status", web::get().to(get_status))
            .route("/upload-sessions", web::post().to(create_upload_session)),
    );
}

async fn get_status(
    service: web::Data<OnedriveService>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(err) = middleware::verify_bearer_token(&req, &["upload:read"]) {
        return map_cli_auth_error(err);
    }

    match service.status().await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => map_onedrive_error(err),
    }
}

async fn create_upload_session(
    service: web::Data<OnedriveService>,
    req: HttpRequest,
    payload: web::Json<CreateUploadSessionRequest>,
) -> impl Responder {
    if let Err(err) = middleware::verify_bearer_token(&req, &["upload:create"]) {
        return map_cli_auth_error(err);
    }

    let conflict_behavior = payload
        .conflict_behavior
        .as_deref()
        .unwrap_or("replace");

    match service
        .create_upload_session(
            &payload.path,
            payload.file_name.as_deref(),
            payload.file_size,
            conflict_behavior,
        )
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => map_onedrive_error(err),
    }
}

fn map_cli_auth_error(err: crate::cli_auth::service::ServiceError) -> HttpResponse {
    match err {
        crate::cli_auth::service::ServiceError::Unauthorized(message) => {
            HttpResponse::Unauthorized().json(ErrorResponse { error: message })
        }
        crate::cli_auth::service::ServiceError::BadRequest(message) => {
            HttpResponse::BadRequest().json(ErrorResponse { error: message })
        }
        crate::cli_auth::service::ServiceError::InvalidClientId => HttpResponse::BadRequest()
            .json(ErrorResponse {
                error: "client_id 不被允许".to_string(),
            }),
        crate::cli_auth::service::ServiceError::Conflict(message) => {
            HttpResponse::Conflict().json(ErrorResponse { error: message })
        }
        crate::cli_auth::service::ServiceError::Config(message)
        | crate::cli_auth::service::ServiceError::Internal(message) => {
            HttpResponse::InternalServerError().json(ErrorResponse { error: message })
        }
    }
}

fn map_onedrive_error(err: OnedriveError) -> HttpResponse {
    match err {
        OnedriveError::BadRequest(message) => {
            HttpResponse::BadRequest().json(ErrorResponse { error: message })
        }
        OnedriveError::Unauthorized(message) => {
            HttpResponse::Unauthorized().json(ErrorResponse { error: message })
        }
        OnedriveError::Forbidden(message) => {
            HttpResponse::Forbidden().json(ErrorResponse { error: message })
        }
        OnedriveError::Conflict(message) => {
            HttpResponse::Conflict().json(ErrorResponse { error: message })
        }
        OnedriveError::Upstream(message) => {
            HttpResponse::BadGateway().json(ErrorResponse { error: message })
        }
        OnedriveError::Config(message) | OnedriveError::Internal(message) => {
            HttpResponse::InternalServerError().json(ErrorResponse { error: message })
        }
    }
}
