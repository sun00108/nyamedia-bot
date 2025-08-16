CREATE TABLE media_requests (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,  -- 'TMDB' or 'BGM.TV'
    media_id TEXT NOT NULL,
    request_user BIGINT NOT NULL, -- telegram_id of requesting user
    status INTEGER NOT NULL DEFAULT 0, -- 0: 已提交, 1: 已入库, 2: 被取消, 3: 不符合规范
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source, media_id)
);
