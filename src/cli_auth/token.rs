use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCodeClaims {
    pub kind: String,
    pub state: String,
    pub client_id: String,
    pub telegram_user_id: i64,
    pub telegram_username: String,
    pub jti: String,
    pub exp: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliAccessTokenClaims {
    pub kind: String,
    pub sub: String,
    pub telegram_user_id: i64,
    pub telegram_username: String,
    pub scope: Vec<String>,
    pub iat: usize,
    pub exp: usize,
}

pub fn issue_authorization_code(
    secret: &str,
    state: &str,
    client_id: &str,
    telegram_user_id: i64,
    telegram_username: &str,
    ttl_secs: i64,
) -> Result<(String, AuthorizationCodeClaims, DateTime<Utc>), String> {
    let expires_at = Utc::now() + Duration::seconds(ttl_secs);
    let claims = AuthorizationCodeClaims {
        kind: "cli_authorization_code".to_string(),
        state: state.to_string(),
        client_id: client_id.to_string(),
        telegram_user_id,
        telegram_username: telegram_username.to_string(),
        jti: Uuid::new_v4().to_string(),
        exp: expires_at.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| err.to_string())?;

    Ok((token, claims, expires_at))
}

pub fn decode_authorization_code(
    secret: &str,
    token: &str,
) -> Result<AuthorizationCodeClaims, String> {
    let data = decode::<AuthorizationCodeClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|err| err.to_string())?;

    Ok(data.claims)
}

pub fn issue_cli_access_token(
    secret: &str,
    telegram_user_id: i64,
    telegram_username: &str,
    scopes: &[String],
    ttl_secs: i64,
) -> Result<(String, CliAccessTokenClaims, DateTime<Utc>, DateTime<Utc>), String> {
    let created_at = Utc::now();
    let expires_at = created_at + Duration::seconds(ttl_secs);
    let claims = CliAccessTokenClaims {
        kind: "cli_access_token".to_string(),
        sub: telegram_user_id.to_string(),
        telegram_user_id,
        telegram_username: telegram_username.to_string(),
        scope: scopes.to_vec(),
        iat: created_at.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| err.to_string())?;

    Ok((token, claims, created_at, expires_at))
}

pub fn decode_cli_access_token(secret: &str, token: &str) -> Result<CliAccessTokenClaims, String> {
    let data = decode::<CliAccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|err| err.to_string())?;

    Ok(data.claims)
}
