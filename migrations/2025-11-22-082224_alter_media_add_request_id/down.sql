-- Revert to previous media table structure
DROP TABLE media;

CREATE TABLE media (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    media_id TEXT NOT NULL,
    source TEXT NOT NULL, -- "tmdb" or "bgm"
    media_type TEXT NOT NULL, -- "movie", "tv" for tmdb; "subject" for bgm
    title TEXT NOT NULL,
    summary TEXT,
    poster TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(media_id, source)
);