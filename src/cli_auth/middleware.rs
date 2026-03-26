use actix_web::HttpRequest;

use super::service::{CliAuthConfig, ServiceError};
use super::token::{self, CliAccessTokenClaims};

pub fn verify_bearer_token(
    req: &HttpRequest,
    required_scopes: &[&str],
) -> Result<CliAccessTokenClaims, ServiceError> {
    let config = CliAuthConfig::from_env()?;
    let header = req
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ServiceError::Unauthorized("缺少 Authorization 头".to_string()))?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ServiceError::Unauthorized("Authorization 头格式错误".to_string()))?;

    let claims = token::decode_cli_access_token(&config.access_token_secret, token)
        .map_err(|_| ServiceError::Unauthorized("access token 无效或已过期".to_string()))?;

    if claims.kind != "cli_access_token" {
        return Err(ServiceError::Unauthorized("access token 类型错误".to_string()));
    }

    for scope in required_scopes {
        if !claims.scope.iter().any(|item| item == scope) {
            return Err(ServiceError::Unauthorized(format!(
                "缺少所需权限: {}",
                scope
            )));
        }
    }

    Ok(claims)
}
