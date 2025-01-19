CREATE TABLE telegram_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    telegram_id BIGINT NOT NULL,
    username VARCHAR(255) NOT NULL,
    emby_user_id VARCHAR(255) NOT NULL,
    admin BOOLEAN DEFAULT 0 NOT NULL,
    UNIQUE(telegram_id)
);