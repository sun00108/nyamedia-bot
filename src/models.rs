use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::telegram_users)] // 可选的，但是极大地改善了生成的编译器错误信息。
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TelegramUser {
    pub id: i32,
    pub telegram_id: i64,
    pub username: String,
    pub admin: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::telegram_users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTelegramUser {
    pub telegram_id: i64,
    pub username: String,
    pub emby_user_id: String,
}