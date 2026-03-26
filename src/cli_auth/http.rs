use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use super::service::{self, RequestContext, ServiceError, TelegramLoginPayload};

#[derive(Debug, Deserialize)]
struct ChallengeQuery {
    client_id: String,
    state: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyTelegramLoginRequest {
    client_id: String,
    state: String,
    telegram_login: TelegramLoginPayload,
}

#[derive(Debug, Deserialize)]
struct ExchangeRequest {
    client_id: String,
    state: String,
    authorization_code: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/cli/login")
            .route("/challenge", web::get().to(init_challenge))
            .route("/telegram/verify", web::post().to(verify_telegram_login))
            .route("/exchange", web::post().to(exchange_authorization_code)),
    );
}

async fn init_challenge(
    query: web::Query<ChallengeQuery>,
    req: HttpRequest,
) -> impl Responder {
    match service::create_or_get_challenge(
        &query.client_id,
        &query.state,
        query.source.as_deref(),
        request_context(&req),
    ) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => map_error(err),
    }
}

async fn verify_telegram_login(
    payload: web::Json<VerifyTelegramLoginRequest>,
    req: HttpRequest,
) -> impl Responder {
    match service::verify_telegram_login(
        &payload.client_id,
        &payload.state,
        payload.telegram_login.clone(),
        request_context(&req),
    ) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => map_error(err),
    }
}

async fn exchange_authorization_code(payload: web::Json<ExchangeRequest>) -> impl Responder {
    match service::exchange_authorization_code(
        &payload.client_id,
        &payload.state,
        &payload.authorization_code,
    ) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => map_error(err),
    }
}

fn request_context(req: &HttpRequest) -> RequestContext {
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .map(ToOwned::to_owned);
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);

    RequestContext { ip, user_agent }
}

fn map_error(err: ServiceError) -> HttpResponse {
    match err {
        ServiceError::BadRequest(message) => {
            HttpResponse::BadRequest().json(ErrorResponse { error: message })
        }
        ServiceError::Unauthorized(message) => {
            HttpResponse::Unauthorized().json(ErrorResponse { error: message })
        }
        ServiceError::Conflict(message) => {
            HttpResponse::Conflict().json(ErrorResponse { error: message })
        }
        ServiceError::InvalidClientId => HttpResponse::BadRequest().json(ErrorResponse {
            error: "client_id 不被允许".to_string(),
        }),
        ServiceError::Config(message) | ServiceError::Internal(message) => {
            HttpResponse::InternalServerError().json(ErrorResponse { error: message })
        }
    }
}
