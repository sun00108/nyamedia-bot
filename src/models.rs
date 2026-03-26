use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::cli_login_challenges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CliLoginChallenge {
    pub id: i32,
    pub state: String,
    pub client_id: String,
    pub status: String,
    pub source: Option<String>,
    pub telegram_user_id: Option<i64>,
    pub telegram_username: Option<String>,
    pub authorization_code_jti: Option<String>,
    pub request_ip: Option<String>,
    pub completed_ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub expires_at: String,
    pub consumed_at: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::cli_login_challenges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewCliLoginChallenge {
    pub state: String,
    pub client_id: String,
    pub status: String,
    pub source: Option<String>,
    pub telegram_user_id: Option<i64>,
    pub telegram_username: Option<String>,
    pub authorization_code_jti: Option<String>,
    pub request_ip: Option<String>,
    pub completed_ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub expires_at: String,
    pub consumed_at: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::telegram_users)] // 可选的，但是极大地改善了生成的编译器错误信息。
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TelegramUser {
    pub id: i32,
    pub telegram_id: i64,
    pub username: String,
    pub admin: bool,
    pub emby_user_id: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::telegram_users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTelegramUser {
    pub telegram_id: i64,
    pub username: String,
    pub emby_user_id: String,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::media_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct MediaRequest {
    pub id: i32,
    pub source: String,
    pub media_id: String,
    pub request_user: i64,
    pub status: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::media_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewMediaRequest {
    pub source: String,
    pub media_id: String,
    pub request_user: i64,
    pub status: i32,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::media)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Media {
    pub id: i32,
    pub media_request_id: i32,
    pub title: String,
    pub summary: Option<String>,
    pub poster: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::media)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewMedia {
    pub media_request_id: i32,
    pub title: String,
    pub summary: Option<String>,
    pub poster: Option<String>,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::media_upload_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct MediaUploadRequest {
    pub id: i32,
    pub media_request_id: i32,
    pub request_user: i64,
    pub request_code: String,
    pub media_title: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub target_path: String,
    pub status: String,
    pub uploaded_file_name: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::media_upload_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewMediaUploadRequest {
    pub media_request_id: i32,
    pub request_user: i64,
    pub request_code: String,
    pub media_title: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub target_path: String,
    pub status: String,
    pub uploaded_file_name: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::media_upload_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateMediaUploadRequest {
    pub media_title: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub target_path: Option<String>,
    pub status: Option<String>,
    pub uploaded_file_name: Option<String>,
    pub completed_at: Option<String>,
}

// Status constants for MediaRequest
pub mod media_request_status {
    pub const SUBMITTED: i32 = 0;   // 已提交
    pub const ARCHIVED: i32 = 1;    // 已入库
    pub const CANCELLED: i32 = 2;   // 被取消
    pub const INVALID: i32 = 3;     // 不符合规范
}

pub mod cli_login_challenge_status {
    pub const PENDING: &str = "pending";
    pub const COMPLETED: &str = "completed";
    pub const CONSUMED: &str = "consumed";
    pub const EXPIRED: &str = "expired";
}

pub mod media_upload_request_status {
    pub const PENDING: &str = "pending";
    pub const COMPLETED: &str = "completed";
    pub const CONSUMED: &str = "consumed";
}
