use crate::establish_connection;
use diesel::prelude::*;
use crate::models::{NewTelegramUser, TelegramUser};

pub fn check_admin(tg_id: i64) -> bool {
    use crate::schema::telegram_users::dsl::*;
    let conn = &mut establish_connection();
    let results = telegram_users
        .filter(telegram_id.eq(tg_id))
        .limit(1)
        .load::<TelegramUser>(conn)
        .expect("Error loading users");
    if results.len() > 0 {
        return results[0].admin;
    }
    false
}

pub fn check_registered(tg_id: i64) -> bool {
    use crate::schema::telegram_users::dsl::*;
    let conn = &mut establish_connection();
    let results = telegram_users
        .filter(telegram_id.eq(tg_id))
        .limit(1)
        .load::<TelegramUser>(conn)
        .expect("Error loading users");
    if results.len() > 0 {
        return true;
    }
    false
}

pub fn register(telegram_id: i64, username: String, emby_user_id: String) {
    use crate::schema::telegram_users;
    let conn = &mut establish_connection();
    let new_user = NewTelegramUser {
        telegram_id,
        username,
        emby_user_id
    };
    diesel::insert_into(telegram_users::table)
        .values(&new_user)
        .execute(conn)
        .expect("Error registering user");
}

pub fn delete_user(tg_id: i64) -> Result<(), String> {
    use crate::schema::telegram_users::dsl::*;
    let conn = &mut establish_connection();
    let deleted = diesel::delete(telegram_users.filter(telegram_id.eq(tg_id)))
        .execute(conn)
        .map_err(|_| "数据库操作失败，请联系管理员。".to_string())?;

    if deleted > 0 {
        Ok(())
    } else {
        Err("找不到用户记录，可能已经被删除。".to_string())
    }
}

pub fn get_emby_id(tg_id: i64) -> String {
    use crate::schema::telegram_users::dsl::*;
    let conn = &mut establish_connection();
    let results = telegram_users
        .filter(telegram_id.eq(tg_id))
        .limit(1)
        .load::<TelegramUser>(conn)
        .expect("Error loading users");
    if results.len() > 0 {
        return results[0].emby_user_id.clone();
    }
    "".to_string()
}

pub fn get_username(tg_id: i64) -> String {
    use crate::schema::telegram_users::dsl::*;
    let conn = &mut establish_connection();
    let results = telegram_users
        .filter(telegram_id.eq(tg_id))
        .limit(1)
        .load::<TelegramUser>(conn)
        .expect("Error loading users");
    if results.len() > 0 {
        return results[0].username.clone();
    }
    "".to_string()
}