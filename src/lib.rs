pub mod schema;
pub mod models;

pub mod auth;
pub mod bot;
pub mod webhook;

use std::env;
use diesel::{Connection, SqliteConnection};
use dotenvy::dotenv;
use reqwest::Client;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub async fn delete_emby_user(user_id: &str) -> Result<(), String> {
    let client = Client::new();
    let emby_url = env::var("EMBY_URL").expect("EMBY_URL must be set");
    let emby_token = env::var("EMBY_TOKEN").expect("EMBY_TOKEN must be set");

    let res = client.delete(format!("{}/Users/{}", emby_url, user_id))
        .header("X-Emby-Token", emby_token)
        .send()
        .await
        .map_err(|_| "请联系管理员。[Error: Emby API]".to_string())?;

    if res.status().is_success() {
        Ok(())
    } else {
        let error_message = res.text().await.map_err(|_| "请联系管理员。[Error: res.text]".to_string())?;
        Err(error_message)
    }
}