use crate::establish_connection;
use std::env;
use diesel::prelude::*;
use dotenvy::dotenv;
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

pub fn register(telegram_id: i64, username: String) {
    use crate::schema::telegram_users;
    let conn = &mut establish_connection();
    let new_user = NewTelegramUser {
        telegram_id,
        username,
    };
    diesel::insert_into(telegram_users::table)
        .values(&new_user)
        .execute(conn)
        .expect("Error registering user");
}