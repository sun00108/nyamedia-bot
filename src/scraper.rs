use std::env;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    pub title: String,
    pub summary: String,
    pub poster: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmdbResponse {
    title: Option<String>,
    name: Option<String>, // for TV shows
    overview: String,
    poster_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BgmResponse {
    name_cn: String,
    summary: String,
    images: BgmImages,
}

#[derive(Debug, Serialize, Deserialize)]
struct BgmImages {
    common: String,
}

pub async fn scrape_media_info(source: &str, media_type: &str, media_id: &str) -> Result<MediaInfo, String> {
    match source.to_lowercase().as_str() {
        "tmdb" => scrape_tmdb(media_type, media_id).await,
        "bgm" => scrape_bgm(media_id).await,
        _ => Err("Unsupported media source".to_string()),
    }
}

async fn scrape_tmdb(media_type: &str, media_id: &str) -> Result<MediaInfo, String> {
    let access_token = env::var("TMDB_ACCESS_TOKEN")
        .map_err(|_| "TMDB_ACCESS_TOKEN not found in environment".to_string())?;
    
    let client = Client::new();
    let url = format!("https://api.themoviedb.org/3/{}/{}?language=zh-CN", media_type, media_id);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch from TMDB: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("TMDB API returned status: {}", response.status()));
    }

    let tmdb_data: TmdbResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse TMDB response: {}", e))?;

    let title = tmdb_data.title
        .or(tmdb_data.name)
        .unwrap_or_else(|| "Unknown Title".to_string());
    
    let poster = tmdb_data.poster_path
        .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
        .unwrap_or_else(|| "".to_string());

    Ok(MediaInfo {
        title,
        summary: tmdb_data.overview,
        poster,
    })
}

async fn scrape_bgm(media_id: &str) -> Result<MediaInfo, String> {
    let access_token = env::var("BGM_ACCESS_TOKEN")
        .map_err(|_| "BGM_ACCESS_TOKEN not found in environment".to_string())?;
    
    let client = Client::new();
    let url = format!("https://api.bgm.tv/v0/subjects/{}", media_id);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "sun00108/nyamedia-bot")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch from BGM: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("BGM API returned status: {}", response.status()));
    }

    let bgm_data: BgmResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse BGM response: {}", e))?;

    Ok(MediaInfo {
        title: bgm_data.name_cn,
        summary: bgm_data.summary,
        poster: bgm_data.images.common,
    })
}

// 保存媒体信息到数据库
pub fn save_media_to_db(
    conn: &mut diesel::SqliteConnection,
    media_request_id: i32,
    media_info: &MediaInfo,
) -> Result<(), diesel::result::Error> {
    use crate::models::NewMedia;
    use crate::schema::media;
    use diesel::prelude::*;

    let new_media = NewMedia {
        media_request_id,
        title: media_info.title.clone(),
        summary: if media_info.summary.is_empty() { None } else { Some(media_info.summary.clone()) },
        poster: if media_info.poster.is_empty() { None } else { Some(media_info.poster.clone()) },
    };

    // 插入或更新（基于unique的media_request_id）
    diesel::insert_into(media::table)
        .values(&new_media)
        .on_conflict(media::media_request_id)
        .do_update()
        .set((
            media::title.eq(&new_media.title),
            media::summary.eq(&new_media.summary),
            media::poster.eq(&new_media.poster),
            media::updated_at.eq(diesel::dsl::now),
        ))
        .execute(conn)?;
    
    Ok(())
}