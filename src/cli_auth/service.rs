use std::collections::HashSet;
use std::env;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use diesel::prelude::*;
use diesel::OptionalExtension;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::database;
use crate::models::{
    cli_login_challenge_status, CliLoginChallenge, NewCliLoginChallenge, TelegramUser,
};
use crate::schema::{cli_login_challenges, telegram_users};

use super::token;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct CliAuthConfig {
    pub allowed_client_ids: HashSet<String>,
    pub code_secret: String,
    pub access_token_secret: String,
    pub challenge_ttl_secs: i64,
    pub code_ttl_secs: i64,
    pub access_token_ttl_secs: i64,
    pub telegram_auth_max_age_secs: i64,
    pub access_token_scopes: Vec<String>,
    pub bot_token: String,
}

impl CliAuthConfig {
    pub fn from_env() -> Result<Self, ServiceError> {
        let allowed_client_ids = env::var("CLI_AUTH_ALLOWED_CLIENT_IDS")
            .unwrap_or_else(|_| "nyaupload-cli".to_string())
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<HashSet<_>>();

        if allowed_client_ids.is_empty() {
            return Err(ServiceError::Config(
                "CLI_AUTH_ALLOWED_CLIENT_IDS 不能为空".to_string(),
            ));
        }

        Ok(Self {
            allowed_client_ids,
            code_secret: env::var("CLI_AUTH_CODE_SECRET")
                .map_err(|_| ServiceError::Config("CLI_AUTH_CODE_SECRET 未配置".to_string()))?,
            access_token_secret: env::var("CLI_ACCESS_TOKEN_SECRET").map_err(|_| {
                ServiceError::Config("CLI_ACCESS_TOKEN_SECRET 未配置".to_string())
            })?,
            challenge_ttl_secs: parse_env_i64("CLI_AUTH_CHALLENGE_TTL_SECONDS", 600)?,
            code_ttl_secs: parse_env_i64("CLI_AUTH_CODE_TTL_SECONDS", 180)?,
            access_token_ttl_secs: parse_env_i64("CLI_ACCESS_TOKEN_TTL_SECONDS", 2_592_000)?,
            telegram_auth_max_age_secs: parse_env_i64("TELEGRAM_AUTH_MAX_AGE_SECONDS", 300)?,
            access_token_scopes: env::var("CLI_ACCESS_TOKEN_SCOPES")
                .unwrap_or_else(|_| "upload:create,upload:read".to_string())
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>(),
            bot_token: env::var("TELOXIDE_TOKEN")
                .map_err(|_| ServiceError::Config("TELOXIDE_TOKEN 未配置".to_string()))?,
        })
    }

    pub fn validate_client_id(&self, client_id: &str) -> Result<(), ServiceError> {
        if self.allowed_client_ids.contains(client_id) {
            Ok(())
        } else {
            Err(ServiceError::InvalidClientId)
        }
    }
}

#[derive(Debug)]
pub enum ServiceError {
    Config(String),
    BadRequest(String),
    Unauthorized(String),
    Conflict(String),
    Internal(String),
    InvalidClientId,
}

#[derive(Debug, Serialize)]
pub struct ChallengeInitResult {
    pub state: String,
    pub client_id: String,
    pub status: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramLoginPayload {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
}

#[derive(Debug)]
pub struct RequestContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyTelegramResult {
    pub authorization_code: String,
    pub expires_at: String,
    pub created_at: String,
    pub telegram_id: i64,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeResult {
    pub access_token: String,
    pub token_type: String,
    pub username: String,
    pub telegram_id: i64,
    pub expires_at: String,
    pub created_at: String,
}

pub fn create_or_get_challenge(
    client_id: &str,
    state: &str,
    source: Option<&str>,
    context: RequestContext,
) -> Result<ChallengeInitResult, ServiceError> {
    let config = CliAuthConfig::from_env()?;
    config.validate_client_id(client_id)?;

    if state.trim().is_empty() {
        return Err(ServiceError::BadRequest("state 不能为空".to_string()));
    }

    let mut conn = database::establish_connection()
        .map_err(|err| ServiceError::Internal(format!("数据库连接失败: {}", err)))?;
    let now = Utc::now();

    if let Some(existing) = cli_login_challenges::table
        .filter(cli_login_challenges::state.eq(state))
        .first::<CliLoginChallenge>(&mut conn)
        .optional()
        .map_err(map_db_err)?
    {
        let expires_at = parse_timestamp(&existing.expires_at)?;

        if existing.client_id != client_id {
            return Err(ServiceError::Conflict("state 已被其他 client_id 使用".to_string()));
        }

        if existing.status == cli_login_challenge_status::CONSUMED || existing.consumed_at.is_some() {
            return Err(ServiceError::Conflict("该登录流程已完成，不能重复使用".to_string()));
        }

        if expires_at <= now {
            expire_challenge(&mut conn, existing.id, now)?;
            return Err(ServiceError::Conflict("该登录流程已过期，请重新发起登录".to_string()));
        }

        return Ok(ChallengeInitResult {
            state: existing.state,
            client_id: existing.client_id,
            status: existing.status,
            expires_at: existing.expires_at,
        });
    }

    let expires_at = now + Duration::seconds(config.challenge_ttl_secs);
    let new_challenge = NewCliLoginChallenge {
        state: state.to_string(),
        client_id: client_id.to_string(),
        status: cli_login_challenge_status::PENDING.to_string(),
        source: source.map(ToOwned::to_owned),
        telegram_user_id: None,
        telegram_username: None,
        authorization_code_jti: None,
        request_ip: context.ip,
        completed_ip: None,
        user_agent: context.user_agent,
        created_at: timestamp_string(now),
        completed_at: None,
        expires_at: timestamp_string(expires_at),
        consumed_at: None,
    };

    diesel::insert_into(cli_login_challenges::table)
        .values(&new_challenge)
        .execute(&mut conn)
        .map_err(map_db_err)?;

    Ok(ChallengeInitResult {
        state: state.to_string(),
        client_id: client_id.to_string(),
        status: cli_login_challenge_status::PENDING.to_string(),
        expires_at: timestamp_string(expires_at),
    })
}

pub fn verify_telegram_login(
    client_id: &str,
    state: &str,
    telegram_login: TelegramLoginPayload,
    context: RequestContext,
) -> Result<VerifyTelegramResult, ServiceError> {
    let config = CliAuthConfig::from_env()?;
    config.validate_client_id(client_id)?;

    verify_telegram_payload(&config, &telegram_login)?;

    let mut conn = database::establish_connection()
        .map_err(|err| ServiceError::Internal(format!("数据库连接失败: {}", err)))?;
    let now = Utc::now();

    let challenge = cli_login_challenges::table
        .filter(cli_login_challenges::state.eq(state))
        .first::<CliLoginChallenge>(&mut conn)
        .optional()
        .map_err(map_db_err)?
        .ok_or_else(|| ServiceError::BadRequest("未找到对应的登录 challenge".to_string()))?;

    if challenge.client_id != client_id {
        return Err(ServiceError::Conflict("challenge 与 client_id 不匹配".to_string()));
    }

    if challenge.status == cli_login_challenge_status::CONSUMED || challenge.consumed_at.is_some() {
        return Err(ServiceError::Conflict("该登录流程已被使用".to_string()));
    }

    let challenge_expires_at = parse_timestamp(&challenge.expires_at)?;
    if challenge_expires_at <= now {
        expire_challenge(&mut conn, challenge.id, now)?;
        return Err(ServiceError::Conflict("登录 challenge 已过期".to_string()));
    }

    let registered_user = telegram_users::table
        .filter(telegram_users::telegram_id.eq(telegram_login.id))
        .first::<TelegramUser>(&mut conn)
        .optional()
        .map_err(map_db_err)?;

    if registered_user.is_none() {
        return Err(ServiceError::Unauthorized("该 Telegram 用户未注册，不能登录 CLI".to_string()));
    }

    let display_username = telegram_login
        .username
        .clone()
        .unwrap_or_else(|| telegram_login.first_name.clone());

    let (authorization_code, claims, code_expires_at) = token::issue_authorization_code(
        &config.code_secret,
        state,
        client_id,
        telegram_login.id,
        &display_username,
        config.code_ttl_secs,
    )
    .map_err(ServiceError::Internal)?;

    diesel::update(cli_login_challenges::table.filter(cli_login_challenges::id.eq(challenge.id)))
        .set((
            cli_login_challenges::status.eq(cli_login_challenge_status::COMPLETED),
            cli_login_challenges::telegram_user_id.eq(Some(telegram_login.id)),
            cli_login_challenges::telegram_username.eq(Some(display_username.clone())),
            cli_login_challenges::authorization_code_jti.eq(Some(claims.jti)),
            cli_login_challenges::completed_at.eq(Some(timestamp_string(now))),
            cli_login_challenges::completed_ip.eq(context.ip),
            cli_login_challenges::expires_at.eq(timestamp_string(code_expires_at)),
        ))
        .execute(&mut conn)
        .map_err(map_db_err)?;

    Ok(VerifyTelegramResult {
        authorization_code,
        expires_at: timestamp_string(code_expires_at),
        created_at: timestamp_string(now),
        telegram_id: telegram_login.id,
        username: display_username,
    })
}

pub fn exchange_authorization_code(
    client_id: &str,
    state: &str,
    authorization_code: &str,
) -> Result<ExchangeResult, ServiceError> {
    let config = CliAuthConfig::from_env()?;
    config.validate_client_id(client_id)?;

    let claims =
        token::decode_authorization_code(&config.code_secret, authorization_code).map_err(|_| {
            ServiceError::Unauthorized("authorization code 无效或已过期".to_string())
        })?;

    if claims.kind != "cli_authorization_code" {
        return Err(ServiceError::Unauthorized("authorization code 类型错误".to_string()));
    }

    if claims.client_id != client_id || claims.state != state {
        return Err(ServiceError::Unauthorized("authorization code 与请求参数不匹配".to_string()));
    }

    let mut conn = database::establish_connection()
        .map_err(|err| ServiceError::Internal(format!("数据库连接失败: {}", err)))?;
    let now = Utc::now();

    let challenge = cli_login_challenges::table
        .filter(cli_login_challenges::state.eq(state))
        .first::<CliLoginChallenge>(&mut conn)
        .optional()
        .map_err(map_db_err)?
        .ok_or_else(|| ServiceError::BadRequest("未找到对应的登录 challenge".to_string()))?;

    if challenge.client_id != client_id {
        return Err(ServiceError::Conflict("challenge 与 client_id 不匹配".to_string()));
    }

    if challenge.status == cli_login_challenge_status::CONSUMED || challenge.consumed_at.is_some() {
        return Err(ServiceError::Conflict("authorization code 已被使用".to_string()));
    }

    if challenge.status != cli_login_challenge_status::COMPLETED {
        return Err(ServiceError::BadRequest("登录尚未完成，不能兑换 access token".to_string()));
    }

    let challenge_expires_at = parse_timestamp(&challenge.expires_at)?;
    if challenge_expires_at <= now {
        expire_challenge(&mut conn, challenge.id, now)?;
        return Err(ServiceError::Unauthorized("authorization code 已过期".to_string()));
    }

    if challenge.authorization_code_jti.as_deref() != Some(claims.jti.as_str()) {
        return Err(ServiceError::Unauthorized("authorization code 已失效".to_string()));
    }

    let telegram_username = challenge
        .telegram_username
        .clone()
        .ok_or_else(|| ServiceError::Internal("challenge 缺少 telegram_username".to_string()))?;
    let telegram_user_id = challenge
        .telegram_user_id
        .ok_or_else(|| ServiceError::Internal("challenge 缺少 telegram_user_id".to_string()))?;

    diesel::update(cli_login_challenges::table.filter(cli_login_challenges::id.eq(challenge.id)))
        .set((
            cli_login_challenges::status.eq(cli_login_challenge_status::CONSUMED),
            cli_login_challenges::consumed_at.eq(Some(timestamp_string(now))),
        ))
        .execute(&mut conn)
        .map_err(map_db_err)?;

    let (access_token, _claims, created_at, expires_at) = token::issue_cli_access_token(
        &config.access_token_secret,
        telegram_user_id,
        &telegram_username,
        &config.access_token_scopes,
        config.access_token_ttl_secs,
    )
    .map_err(ServiceError::Internal)?;

    Ok(ExchangeResult {
        access_token,
        token_type: "Bearer".to_string(),
        username: telegram_username,
        telegram_id: telegram_user_id,
        expires_at: timestamp_string(expires_at),
        created_at: timestamp_string(created_at),
    })
}

fn verify_telegram_payload(
    config: &CliAuthConfig,
    payload: &TelegramLoginPayload,
) -> Result<(), ServiceError> {
    let now = Utc::now().timestamp();
    if payload.auth_date > now + 30 {
        return Err(ServiceError::Unauthorized("Telegram 登录时间异常".to_string()));
    }

    if now - payload.auth_date > config.telegram_auth_max_age_secs {
        return Err(ServiceError::Unauthorized("Telegram 登录已过期，请重新登录".to_string()));
    }

    let expected_hash = telegram_hash(&config.bot_token, payload)?;
    if expected_hash != payload.hash {
        return Err(ServiceError::Unauthorized("Telegram 登录校验失败".to_string()));
    }

    Ok(())
}

fn telegram_hash(bot_token: &str, payload: &TelegramLoginPayload) -> Result<String, ServiceError> {
    let mut entries = vec![
        format!("auth_date={}", payload.auth_date),
        format!("first_name={}", payload.first_name),
        format!("id={}", payload.id),
    ];

    if let Some(last_name) = &payload.last_name {
        if !last_name.is_empty() {
            entries.push(format!("last_name={}", last_name));
        }
    }
    if let Some(photo_url) = &payload.photo_url {
        if !photo_url.is_empty() {
            entries.push(format!("photo_url={}", photo_url));
        }
    }
    if let Some(username) = &payload.username {
        if !username.is_empty() {
            entries.push(format!("username={}", username));
        }
    }

    entries.sort();
    let data_check_string = entries.join("\n");

    let secret_key = Sha256::digest(bot_token.as_bytes());
    let mut mac = HmacSha256::new_from_slice(secret_key.as_slice())
        .map_err(|err| ServiceError::Internal(err.to_string()))?;
    mac.update(data_check_string.as_bytes());
    let result = mac.finalize().into_bytes();

    Ok(hex::encode(result))
}

fn parse_env_i64(key: &str, default_value: i64) -> Result<i64, ServiceError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<i64>()
            .map_err(|_| ServiceError::Config(format!("{} 必须是整数", key))),
        Err(_) => Ok(default_value),
    }
}

fn map_db_err(err: diesel::result::Error) -> ServiceError {
    ServiceError::Internal(format!("数据库操作失败: {}", err))
}

fn timestamp_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, ServiceError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| ServiceError::Internal("时间格式错误".to_string()))
}

fn expire_challenge(
    conn: &mut diesel::SqliteConnection,
    challenge_id: i32,
    now: DateTime<Utc>,
) -> Result<(), ServiceError> {
    diesel::update(cli_login_challenges::table.filter(cli_login_challenges::id.eq(challenge_id)))
        .set((
            cli_login_challenges::status.eq(cli_login_challenge_status::EXPIRED),
            cli_login_challenges::expires_at.eq(timestamp_string(now)),
        ))
        .execute(conn)
        .map_err(map_db_err)?;
    Ok(())
}
