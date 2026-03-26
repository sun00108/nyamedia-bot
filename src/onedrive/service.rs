use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OnedriveConfig {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub redirect_uri: String,
    pub graph_base_url: String,
    pub oauth_base_url: String,
    pub session_file: PathBuf,
}

impl OnedriveConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            client_id: env::var("MICROSOFT_CLIENT_ID")
                .map_err(|_| "MICROSOFT_CLIENT_ID 未配置".to_string())?,
            client_secret: env::var("MICROSOFT_CLIENT_SECRET")
                .map_err(|_| "MICROSOFT_CLIENT_SECRET 未配置".to_string())?,
            tenant_id: env::var("MICROSOFT_TENANT_ID")
                .map_err(|_| "MICROSOFT_TENANT_ID 未配置".to_string())?,
            redirect_uri: env::var("MICROSOFT_REDIRECT_URI")
                .map_err(|_| "MICROSOFT_REDIRECT_URI 未配置".to_string())?,
            graph_base_url: env::var("MICROSOFT_GRAPH_BASE_URL")
                .unwrap_or_else(|_| "https://graph.microsoft.com/v1.0".to_string()),
            oauth_base_url: env::var("MICROSOFT_OAUTH_BASE_URL")
                .unwrap_or_else(|_| "https://login.microsoftonline.com".to_string()),
            session_file: PathBuf::from(
                env::var("MICROSOFT_SESSION_FILE")
                    .unwrap_or_else(|_| "onedrive-session.json".to_string()),
            ),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OnedriveSession {
    pub access_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    pub authorization_code: Option<String>,
    pub authorization_state: Option<String>,
}

#[derive(Debug, Clone)]
struct TokenState {
    session: OnedriveSession,
}

#[derive(Clone)]
pub struct OnedriveService {
    client: Client,
    config: OnedriveConfig,
    state: Arc<Mutex<TokenState>>,
}

#[derive(Debug)]
pub enum OnedriveError {
    Config(String),
    BadRequest(String),
    Forbidden(String),
    Unauthorized(String),
    Conflict(String),
    Upstream(String),
    Internal(String),
}

#[derive(Debug, Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
    expires_in: i64,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorizationCodeTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadSessionResult {
    pub upload_url: String,
    pub expiration_date_time: String,
}

#[derive(Debug, Serialize)]
pub struct OnedriveStatus {
    pub connected: bool,
    pub drive_type: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphCreateUploadSessionResponse {
    #[serde(rename = "uploadUrl")]
    upload_url: String,
    #[serde(rename = "expirationDateTime")]
    expiration_date_time: String,
}

#[derive(Debug, Deserialize)]
struct GraphDriveResponse {
    drive_type: Option<String>,
    owner: Option<GraphDriveOwner>,
}

#[derive(Debug, Deserialize)]
struct GraphDriveOwner {
    user: Option<GraphDriveUser>,
}

#[derive(Debug, Deserialize)]
struct GraphDriveUser {
    #[serde(rename = "email")]
    email: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphErrorEnvelope {
    error: Option<GraphErrorObject>,
}

#[derive(Debug, Deserialize)]
struct GraphErrorObject {
    code: Option<String>,
    message: Option<String>,
}

impl OnedriveService {
    pub fn from_env() -> Result<Self, OnedriveError> {
        let config = OnedriveConfig::from_env().map_err(OnedriveError::Config)?;
        let session = load_session_file(&config.session_file)
            .map_err(OnedriveError::Internal)?
            .unwrap_or_default();
        let state = TokenState {
            session,
        };

        Ok(Self {
            client: Client::new(),
            config,
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub async fn prepare_authorization(&self) -> Result<(String, String), OnedriveError> {
        let state = Uuid::new_v4().to_string();
        let scope = "offline_access Files.ReadWrite User.Read";
        let url = format!(
            "{}/{}/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri={}&response_mode=query&scope={}&state={}",
            self.config.oauth_base_url.trim_end_matches('/'),
            self.config.tenant_id,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(&state),
        );
        let mut session = self.session_snapshot().await;
        session.access_token = None;
        session.expires_at = None;
        session.refresh_token = None;
        session.authorization_code = None;
        session.authorization_state = Some(state.clone());
        self.replace_session(session).await?;
        Ok((url, state))
    }

    pub async fn exchange_redirect_url(
        &self,
        redirect_url: &str,
        expected_state: &str,
    ) -> Result<(), OnedriveError> {
        let parsed =
            Url::parse(redirect_url).map_err(|_| OnedriveError::BadRequest("redirect_url 格式无效".to_string()))?;
        let mut code = None;
        let mut returned_state = None;
        let mut error = None;

        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "code" => code = Some(value.into_owned()),
                "state" => returned_state = Some(value.into_owned()),
                "error" => error = Some(value.into_owned()),
                _ => {}
            }
        }

        if let Some(error) = error {
            return Err(OnedriveError::BadRequest(format!(
                "Microsoft OAuth 返回错误: {}",
                error
            )));
        }

        let code = code.ok_or_else(|| OnedriveError::BadRequest("redirect_url 中缺少 code".to_string()))?;
        let returned_state = returned_state
            .ok_or_else(|| OnedriveError::BadRequest("redirect_url 中缺少 state".to_string()))?;

        if returned_state != expected_state {
            return Err(OnedriveError::BadRequest("OAuth state 不匹配".to_string()));
        }

        self.exchange_authorization_code(&code).await
    }

    pub async fn exchange_authorization_code_input(&self, code: &str) -> Result<(), OnedriveError> {
        if code.trim().is_empty() {
            return Err(OnedriveError::BadRequest("authorization code 不能为空".to_string()));
        }

        let decoded = urlencoding::decode(code.trim())
            .map_err(|_| OnedriveError::BadRequest("authorization code URL 解码失败".to_string()))?;

        self.exchange_authorization_code(decoded.as_ref()).await
    }

    async fn exchange_authorization_code(&self, code: &str) -> Result<(), OnedriveError> {
        let token_endpoint = format!(
            "{}/{}/oauth2/v2.0/token",
            self.config.oauth_base_url.trim_end_matches('/'),
            self.config.tenant_id
        );

        let response = self
            .client
            .post(token_endpoint)
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.config.redirect_uri.as_str()),
                ("scope", "offline_access Files.ReadWrite User.Read"),
            ])
            .send()
            .await
            .map_err(|err| OnedriveError::Upstream(format!("请求 Microsoft token 失败: {}", err)))?;

        if !response.status().is_success() {
            return Err(map_graph_error(response.status(), response.text().await.ok()));
        }

        let payload = response
            .json::<AuthorizationCodeTokenResponse>()
            .await
            .map_err(|err| OnedriveError::Upstream(format!("解析 Microsoft token 响应失败: {}", err)))?;

        let refresh_token = payload.refresh_token.ok_or_else(|| {
            OnedriveError::Internal("Microsoft 未返回 refresh_token，请确认 scope 包含 offline_access".to_string())
        })?;

        let session = OnedriveSession {
            access_token: Some(payload.access_token),
            expires_at: Some(Utc::now() + Duration::seconds(payload.expires_in.max(60))),
            refresh_token: Some(refresh_token),
            authorization_code: None,
            authorization_state: None,
        };

        self.replace_session(session).await
    }

    pub async fn create_upload_session(
        &self,
        path: &str,
        file_name: Option<&str>,
        file_size: u64,
        conflict_behavior: &str,
    ) -> Result<UploadSessionResult, OnedriveError> {
        validate_upload_request(path, file_name, file_size, conflict_behavior)?;

        let access_token = self.get_access_token().await?;
        let normalized_path = normalize_onedrive_path(path);
        let graph_path = encode_onedrive_path(&normalized_path);
        let request_file_name = match file_name {
            Some(name) => name.to_string(),
            None => extract_file_name(&normalized_path)?.to_string(),
        };

        let request_url = format!(
            "{}/me/drive/root:{}:/createUploadSession",
            self.config.graph_base_url.trim_end_matches('/'),
            graph_path
        );

        let response = self
            .client
            .post(request_url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({
                "item": {
                    "@microsoft.graph.conflictBehavior": conflict_behavior,
                    "name": request_file_name
                }
            }))
            .send()
            .await
            .map_err(|err| OnedriveError::Upstream(format!("调用 Microsoft Graph 失败: {}", err)))?;

        let status = response.status();
        if status.is_success() {
            let payload = response
                .json::<GraphCreateUploadSessionResponse>()
                .await
                .map_err(|err| OnedriveError::Upstream(format!("解析 upload session 响应失败: {}", err)))?;

            return Ok(UploadSessionResult {
                upload_url: payload.upload_url,
                expiration_date_time: payload.expiration_date_time,
            });
        }

        Err(map_graph_error(status, response.text().await.ok()))
    }

    pub async fn status(&self) -> Result<OnedriveStatus, OnedriveError> {
        if !self.has_refresh_token().await {
            return Ok(OnedriveStatus {
                connected: false,
                drive_type: None,
                owner: None,
            });
        }

        let access_token = self.get_access_token().await?;
        let response = self
            .client
            .get(format!(
                "{}/me/drive",
                self.config.graph_base_url.trim_end_matches('/')
            ))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|err| OnedriveError::Upstream(format!("调用 Microsoft Graph 失败: {}", err)))?;

        if response.status().is_success() {
            let payload = response
                .json::<GraphDriveResponse>()
                .await
                .map_err(|err| OnedriveError::Upstream(format!("解析 OneDrive 状态失败: {}", err)))?;

            let owner = payload
                .owner
                .and_then(|owner| owner.user)
                .and_then(|user| user.email.or(user.display_name));

            return Ok(OnedriveStatus {
                connected: true,
                drive_type: payload.drive_type,
                owner,
            });
        }

        Err(map_graph_error(response.status(), response.text().await.ok()))
    }

    async fn get_access_token(&self) -> Result<String, OnedriveError> {
        let mut state = self.state.lock().await;
        let now = Utc::now();

        if let (Some(access_token), Some(expires_at)) =
            (&state.session.access_token, state.session.expires_at)
        {
            if expires_at > now + Duration::seconds(60) {
                return Ok(access_token.clone());
            }
        }

        let refresh_token = state.session.refresh_token.clone().ok_or_else(|| {
            OnedriveError::Forbidden("OneDrive 尚未登录，请先运行 odlogin 完成授权".to_string())
        })?;

        let token_endpoint = format!(
            "{}/{}/oauth2/v2.0/token",
            self.config.oauth_base_url.trim_end_matches('/'),
            self.config.tenant_id
        );

        let response = self
            .client
            .post(token_endpoint)
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
                ("redirect_uri", self.config.redirect_uri.as_str()),
                ("scope", "offline_access Files.ReadWrite User.Read"),
            ])
            .send()
            .await
            .map_err(|err| OnedriveError::Upstream(format!("刷新 Microsoft token 失败: {}", err)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.ok();
            if should_clear_session(status, body.as_deref()) {
                state.session = OnedriveSession::default();
                save_session_file(&self.config.session_file, &state.session)
                    .map_err(OnedriveError::Internal)?;
                return Err(OnedriveError::Forbidden(
                    "OneDrive 登录已失效，请重新运行 odlogin 完成浏览器登录".to_string(),
                ));
            }

            return Err(map_graph_error(status, body));
        }

        let payload = response
            .json::<TokenRefreshResponse>()
            .await
            .map_err(|err| OnedriveError::Upstream(format!("解析 Microsoft token 响应失败: {}", err)))?;

        if let Some(refresh_token) = payload.refresh_token {
            state.session.refresh_token = Some(refresh_token);
        }

        let expires_at = now + Duration::seconds(payload.expires_in.max(60));
        state.session.access_token = Some(payload.access_token.clone());
        state.session.expires_at = Some(expires_at);
        save_session_file(&self.config.session_file, &state.session)
            .map_err(OnedriveError::Internal)?;

        Ok(payload.access_token)
    }

    pub fn session_file_path(&self) -> &Path {
        &self.config.session_file
    }

    pub async fn session_snapshot(&self) -> OnedriveSession {
        let state = self.state.lock().await;
        state.session.clone()
    }

    async fn has_refresh_token(&self) -> bool {
        let state = self.state.lock().await;
        state.session.refresh_token.is_some()
    }

    async fn replace_session(&self, session: OnedriveSession) -> Result<(), OnedriveError> {
        save_session_file(&self.config.session_file, &session).map_err(OnedriveError::Internal)?;
        let mut state = self.state.lock().await;
        state.session = session;
        Ok(())
    }
}

fn validate_upload_request(
    path: &str,
    file_name: Option<&str>,
    file_size: u64,
    conflict_behavior: &str,
) -> Result<(), OnedriveError> {
    if path.trim().is_empty() || !path.starts_with('/') {
        return Err(OnedriveError::BadRequest(
            "path 必须是以 / 开头的 OneDrive 绝对路径".to_string(),
        ));
    }

    if file_size == 0 {
        return Err(OnedriveError::BadRequest(
            "file_size 必须大于 0".to_string(),
        ));
    }

    if !matches!(conflict_behavior, "replace" | "rename" | "fail") {
        return Err(OnedriveError::BadRequest(
            "conflict_behavior 仅支持 replace、rename、fail".to_string(),
        ));
    }

    if let Some(file_name) = file_name {
        let actual_name = extract_file_name(path)?;
        if actual_name != file_name {
            return Err(OnedriveError::BadRequest(
                "file_name 与 path 最后一级文件名不一致".to_string(),
            ));
        }
    }

    Ok(())
}

fn extract_file_name(path: &str) -> Result<&str, OnedriveError> {
    path.rsplit('/')
        .find(|segment| !segment.is_empty())
        .ok_or_else(|| OnedriveError::BadRequest("path 必须包含文件名".to_string()))
}

fn normalize_onedrive_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed == "/" {
        "/".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn encode_onedrive_path(path: &str) -> String {
    let mut encoded = String::new();
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        encoded.push('/');
        encoded.push_str(&urlencoding::encode(segment));
    }
    encoded
}

fn map_graph_error(status: StatusCode, body: Option<String>) -> OnedriveError {
    let graph_message = body
        .as_deref()
        .and_then(|raw| serde_json::from_str::<GraphErrorEnvelope>(raw).ok())
        .and_then(|payload| payload.error)
        .map(|error| {
            let code = error.code.unwrap_or_else(|| "unknown".to_string());
            let message = error.message.unwrap_or_else(|| "未知错误".to_string());
            format!("{}: {}", code, message)
        })
        .or(body.filter(|value| !value.trim().is_empty()));

    match status {
        StatusCode::UNAUTHORIZED => {
            OnedriveError::Unauthorized(graph_message.unwrap_or_else(|| "Microsoft 凭据无效".to_string()))
        }
        StatusCode::FORBIDDEN => {
            OnedriveError::Forbidden(graph_message.unwrap_or_else(|| "OneDrive 访问被拒绝".to_string()))
        }
        StatusCode::CONFLICT => {
            OnedriveError::Conflict(graph_message.unwrap_or_else(|| "OneDrive 目标路径冲突".to_string()))
        }
        _ => OnedriveError::Upstream(graph_message.unwrap_or_else(|| {
            format!("Microsoft Graph 返回错误状态 {}", status.as_u16())
        })),
    }
}

fn should_clear_session(status: StatusCode, body: Option<&str>) -> bool {
    if status != StatusCode::BAD_REQUEST && status != StatusCode::UNAUTHORIZED {
        return false;
    }

    let Some(raw) = body else {
        return false;
    };

    if let Ok(payload) = serde_json::from_str::<GraphErrorEnvelope>(raw) {
        if let Some(error) = payload.error {
            if let Some(code) = error.code {
                return matches!(
                    code.as_str(),
                    "invalid_grant" | "interaction_required" | "unauthorized_client"
                );
            }
        }
    }

    raw.contains("invalid_grant")
}

fn load_session_file(path: &Path) -> Result<Option<OnedriveSession>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .map_err(|err| format!("读取 OneDrive session 文件失败: {}", err))?;
    let session = serde_json::from_str::<OnedriveSession>(&content)
        .map_err(|err| format!("解析 OneDrive session 文件失败: {}", err))?;
    Ok(Some(session))
}

fn save_session_file(path: &Path, session: &OnedriveSession) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建 OneDrive session 目录失败: {}", err))?;
        }
    }

    let content = serde_json::to_string_pretty(session)
        .map_err(|err| format!("序列化 OneDrive session 失败: {}", err))?;
    fs::write(path, content).map_err(|err| format!("写入 OneDrive session 文件失败: {}", err))
}

pub fn format_cached_expiry(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}
