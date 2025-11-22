use diesel::prelude::*;

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

// Status constants for MediaRequest
pub mod media_request_status {
    pub const SUBMITTED: i32 = 0;   // 已提交
    pub const ARCHIVED: i32 = 1;    // 已入库
    pub const CANCELLED: i32 = 2;   // 被取消
    pub const INVALID: i32 = 3;     // 不符合规范
}