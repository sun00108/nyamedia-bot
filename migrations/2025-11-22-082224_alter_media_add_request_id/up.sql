-- Add media_request_id to establish one-to-one relationship
-- Drop existing table and recreate with proper foreign key relationship
DROP TABLE media;

CREATE TABLE media (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    media_request_id INTEGER NOT NULL UNIQUE, -- One-to-one relationship
    title TEXT NOT NULL,
    summary TEXT,
    poster TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (media_request_id) REFERENCES media_requests (id) ON DELETE CASCADE
);