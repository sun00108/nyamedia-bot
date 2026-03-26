CREATE TABLE media_upload_requests (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    media_request_id INTEGER NOT NULL,
    request_user BIGINT NOT NULL,
    request_code TEXT NOT NULL UNIQUE,
    media_title TEXT NOT NULL,
    season INTEGER,
    episode INTEGER,
    target_path TEXT NOT NULL,
    status TEXT NOT NULL,
    uploaded_file_name TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (media_request_id) REFERENCES media_requests (id) ON DELETE CASCADE
);

CREATE INDEX idx_media_upload_requests_request_user ON media_upload_requests (request_user);
CREATE INDEX idx_media_upload_requests_media_request_id ON media_upload_requests (media_request_id);
CREATE INDEX idx_media_upload_requests_status ON media_upload_requests (status);
