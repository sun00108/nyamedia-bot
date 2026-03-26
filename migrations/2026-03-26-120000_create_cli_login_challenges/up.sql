CREATE TABLE cli_login_challenges (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    state TEXT NOT NULL UNIQUE,
    client_id TEXT NOT NULL,
    status TEXT NOT NULL,
    source TEXT,
    telegram_user_id BIGINT,
    telegram_username TEXT,
    authorization_code_jti TEXT,
    request_ip TEXT,
    completed_ip TEXT,
    user_agent TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    expires_at TEXT NOT NULL,
    consumed_at TEXT
);

CREATE INDEX idx_cli_login_challenges_client_id ON cli_login_challenges (client_id);
CREATE INDEX idx_cli_login_challenges_status ON cli_login_challenges (status);
