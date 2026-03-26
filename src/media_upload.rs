use chrono::{SecondsFormat, Utc};
use diesel::prelude::*;
use diesel::OptionalExtension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database;
use crate::models::{
    media_request_status, media_upload_request_status, Media, MediaRequest, MediaUploadRequest, NewMediaUploadRequest,
    UpdateMediaUploadRequest,
};
use crate::schema::{media, media_requests, media_upload_requests};

#[derive(Debug)]
pub enum MediaUploadError {
    BadRequest(String),
    Conflict(String),
    Internal(String),
}

#[derive(Debug, Deserialize)]
pub struct CreateMediaUploadRequestInput {
    pub request_id: i32,
    pub season: Option<i32>,
    pub episode: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CreateMediaUploadRequestResult {
    pub media_title: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub request_code: String,
    pub target_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMediaUploadSessionInput {
    pub request_code: String,
    pub file_name: String,
    pub file_size: u64,
    pub conflict_behavior: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateMediaUploadSessionResult {
    pub upload_url: String,
    pub expiration_date_time: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteMediaUploadInput {
    pub request_code: String,
    pub file_name: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteMediaUploadResult {
    pub status: String,
}

pub fn create_upload_request(
    cli_user_id: i64,
    input: CreateMediaUploadRequestInput,
) -> Result<CreateMediaUploadRequestResult, MediaUploadError> {
    let mut conn = database::establish_connection()
        .map_err(|err| MediaUploadError::Internal(format!("数据库连接失败: {}", err)))?;

    let request = media_requests::table
        .filter(media_requests::id.eq(input.request_id))
        .first::<MediaRequest>(&mut conn)
        .optional()
        .map_err(map_db_err)?
        .ok_or_else(|| MediaUploadError::BadRequest("media request 不存在".to_string()))?;

    if request.status != media_request_status::SUBMITTED {
        return Err(MediaUploadError::Conflict(
            "该媒体请求当前状态不允许创建上传记录".to_string(),
        ));
    }

    let media_record = media::table
        .filter(media::media_request_id.eq(request.id))
        .first::<Media>(&mut conn)
        .optional()
        .map_err(map_db_err)?
        .ok_or_else(|| MediaUploadError::BadRequest("该 media request 缺少已刮削的媒体信息".to_string()))?;

    let (season, episode) = validate_source_and_episode_fields(
        request.source.as_str(),
        input.season,
        input.episode,
    )?;
    let target_path = build_target_path(&media_record.title, season);

    let existing = if let (Some(season), Some(episode)) = (season, episode) {
        media_upload_requests::table
            .filter(media_upload_requests::media_request_id.eq(request.id))
            .filter(media_upload_requests::season.eq(Some(season)))
            .filter(media_upload_requests::episode.eq(Some(episode)))
            .first::<MediaUploadRequest>(&mut conn)
            .optional()
            .map_err(map_db_err)?
    } else {
        media_upload_requests::table
            .filter(media_upload_requests::media_request_id.eq(request.id))
            .first::<MediaUploadRequest>(&mut conn)
            .optional()
            .map_err(map_db_err)?
    };

    if existing.is_some() {
        return Err(MediaUploadError::Conflict(
            "该媒体请求已存在上传记录，不能重复创建".to_string(),
        ));
    }

    let request_code = generate_request_code();
    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let new_upload_request = NewMediaUploadRequest {
        media_request_id: request.id,
        request_user: cli_user_id,
        request_code: request_code.clone(),
        media_title: media_record.title.clone(),
        season,
        episode,
        target_path: target_path.clone(),
        status: media_upload_request_status::PENDING.to_string(),
        uploaded_file_name: None,
        created_at,
        completed_at: None,
    };

    diesel::insert_into(media_upload_requests::table)
        .values(&new_upload_request)
        .execute(&mut conn)
        .map_err(map_db_err)?;

    Ok(CreateMediaUploadRequestResult {
        media_title: media_record.title,
        season,
        episode,
        request_code,
        target_path,
    })
}

pub fn get_upload_request_for_session(
    input: CreateMediaUploadSessionInput,
) -> Result<(MediaUploadRequest, CreateMediaUploadSessionInput), MediaUploadError> {
    let mut conn = database::establish_connection()
        .map_err(|err| MediaUploadError::Internal(format!("数据库连接失败: {}", err)))?;

    if input.request_code.trim().is_empty() {
        return Err(MediaUploadError::BadRequest(
            "request_code 不能为空".to_string(),
        ));
    }

    if input.file_name.trim().is_empty() {
        return Err(MediaUploadError::BadRequest(
            "file_name 不能为空".to_string(),
        ));
    }

    if input.file_size == 0 {
        return Err(MediaUploadError::BadRequest(
            "file_size 必须大于 0".to_string(),
        ));
    }

    let upload_request = media_upload_requests::table
        .filter(media_upload_requests::request_code.eq(input.request_code.trim()))
        .first::<MediaUploadRequest>(&mut conn)
        .optional()
        .map_err(map_db_err)?
        .ok_or_else(|| MediaUploadError::BadRequest("request_code 无效".to_string()))?;

    if upload_request.status != media_upload_request_status::PENDING {
        return Err(MediaUploadError::Conflict(
            "该 request_code 已不可用".to_string(),
        ));
    }

    let conflict_behavior = input
        .conflict_behavior
        .clone()
        .unwrap_or_else(|| "replace".to_string());
    if !matches!(conflict_behavior.as_str(), "replace" | "rename" | "fail") {
        return Err(MediaUploadError::BadRequest(
            "conflict_behavior 仅支持 replace、rename、fail".to_string(),
        ));
    }

    Ok((
        upload_request,
        CreateMediaUploadSessionInput {
            request_code: input.request_code.trim().to_string(),
            file_name: input.file_name.trim().to_string(),
            file_size: input.file_size,
            conflict_behavior: Some(conflict_behavior),
        },
    ))
}

pub fn complete_upload_request(
    input: CompleteMediaUploadInput,
) -> Result<CompleteMediaUploadResult, MediaUploadError> {
    let mut conn = database::establish_connection()
        .map_err(|err| MediaUploadError::Internal(format!("数据库连接失败: {}", err)))?;

    if input.request_code.trim().is_empty() {
        return Err(MediaUploadError::BadRequest(
            "request_code 不能为空".to_string(),
        ));
    }

    if input.file_name.trim().is_empty() {
        return Err(MediaUploadError::BadRequest(
            "file_name 不能为空".to_string(),
        ));
    }

    let upload_request = media_upload_requests::table
        .filter(media_upload_requests::request_code.eq(input.request_code.trim()))
        .first::<MediaUploadRequest>(&mut conn)
        .optional()
        .map_err(map_db_err)?
        .ok_or_else(|| MediaUploadError::BadRequest("request_code 无效".to_string()))?;

    if upload_request.status != media_upload_request_status::PENDING {
        return Err(MediaUploadError::Conflict(
            "该 request_code 已不可再次完成".to_string(),
        ));
    }

    let completed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let update = UpdateMediaUploadRequest {
        media_title: None,
        season: None,
        episode: None,
        target_path: None,
        status: Some(media_upload_request_status::COMPLETED.to_string()),
        uploaded_file_name: Some(input.file_name.trim().to_string()),
        completed_at: Some(completed_at),
    };

    diesel::update(
        media_upload_requests::table.filter(media_upload_requests::id.eq(upload_request.id)),
    )
    .set(&update)
    .execute(&mut conn)
    .map_err(map_db_err)?;

    Ok(CompleteMediaUploadResult {
        status: "ok".to_string(),
    })
}

fn validate_source_and_episode_fields(
    source: &str,
    season: Option<i32>,
    episode: Option<i32>,
) -> Result<(Option<i32>, Option<i32>), MediaUploadError> {
    match source {
        "TMDB/TV" => {
            let season = season.ok_or_else(|| {
                MediaUploadError::BadRequest("TMDB/TV 请求必须提供 season".to_string())
            })?;
            let episode = episode.ok_or_else(|| {
                MediaUploadError::BadRequest("TMDB/TV 请求必须提供 episode".to_string())
            })?;
            validate_positive_number("season", season)?;
            validate_positive_number("episode", episode)?;
            Ok((Some(season), Some(episode)))
        }
        "BGM.TV" => {
            if let Some(season) = season {
                validate_positive_number("season", season)?;
            }
            if let Some(episode) = episode {
                validate_positive_number("episode", episode)?;
            }

            if season.is_some() ^ episode.is_some() {
                return Err(MediaUploadError::BadRequest(
                    "BGM.TV 如果提供 season/episode，必须同时提供".to_string(),
                ));
            }

            Ok((season, episode))
        }
        _ => {
            if season.is_some() || episode.is_some() {
                return Err(MediaUploadError::BadRequest(
                    "只有 TMDB/TV 或 BGM.TV 可以使用 season/episode".to_string(),
                ));
            }
            Ok((None, None))
        }
    }
}

fn validate_positive_number(name: &str, value: i32) -> Result<(), MediaUploadError> {
    if value <= 0 {
        return Err(MediaUploadError::BadRequest(format!(
            "{} 必须大于 0",
            name
        )));
    }
    Ok(())
}

fn build_target_path(media_title: &str, season: Option<i32>) -> String {
    let safe_title = media_title.trim();
    match season {
        Some(season) => format!("/media/series/{}/Season {:02}/", safe_title, season),
        None => format!("/media/movie/{}/", safe_title),
    }
}

fn generate_request_code() -> String {
    format!("req_{}", Uuid::new_v4().simple())
}

fn map_db_err(err: diesel::result::Error) -> MediaUploadError {
    match err {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => MediaUploadError::Conflict("request_code 已存在，请重试".to_string()),
        other => MediaUploadError::Internal(format!("数据库操作失败: {}", other)),
    }
}
