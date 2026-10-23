CREATE TABLE IF NOT EXISTS users (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    email      TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    avatar     TEXT,
    is_admin   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_identities (
    provider   TEXT NOT NULL,
    subject    TEXT NOT NULL,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (provider, subject),
    UNIQUE (provider, user_id)
);

CREATE TABLE IF NOT EXISTS oauth_states (
    state         TEXT PRIMARY KEY,
    code_verifier TEXT NOT NULL,
    return_to     TEXT NOT NULL DEFAULT '/',
    expires_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS oauth_states_expires_at
    ON oauth_states(expires_at);

CREATE TABLE IF NOT EXISTS runtime_builds (
    id               TEXT PRIMARY KEY,
    runtime          TEXT NOT NULL CHECK (runtime IN ('sgleam', 'spython')),
    display_name     TEXT NOT NULL,
    display_version  TEXT NOT NULL,
    git_commit       TEXT,
    protocol_version INTEGER,
    source_extension TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'development'
                     CHECK (status IN ('development', 'active', 'deprecated', 'unavailable')),
    is_default       INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS runtime_builds_one_default_per_runtime
    ON runtime_builds(runtime)
    WHERE is_default = 1;

INSERT OR IGNORE INTO runtime_builds
    (id, runtime, display_name, display_version, source_extension, status, is_default)
VALUES
    ('sgleam-dev', 'sgleam', 'Sgleam', 'desenvolvimento', '.gleam', 'unavailable', 0),
    ('spython-dev', 'spython', 'Spython', 'desenvolvimento', '.py', 'unavailable', 0);

CREATE TABLE IF NOT EXISTS games (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    thumbnail   BLOB,
    author_id   INTEGER REFERENCES users(id),
    is_featured INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS game_revisions (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id          INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    runtime_build_id TEXT NOT NULL REFERENCES runtime_builds(id),
    source_code      TEXT NOT NULL,
    min_players      INTEGER NOT NULL DEFAULT 1 CHECK (min_players >= 1),
    max_players      INTEGER NOT NULL DEFAULT 1 CHECK (max_players >= min_players),
    status           TEXT NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending', 'approved', 'rejected')),
    review_message   TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at      TEXT
);

CREATE INDEX IF NOT EXISTS game_revisions_game_id
    ON game_revisions(game_id);

CREATE INDEX IF NOT EXISTS game_revisions_status
    ON game_revisions(status);

CREATE INDEX IF NOT EXISTS game_revisions_runtime_build_id
    ON game_revisions(runtime_build_id);

CREATE TABLE IF NOT EXISTS sessions (
    id         TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS sessions_expires_at
    ON sessions(expires_at);
