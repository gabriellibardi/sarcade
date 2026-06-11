CREATE TABLE IF NOT EXISTS users (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    email      TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    avatar     TEXT,
    is_admin   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS games (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    title          TEXT NOT NULL,
    description    TEXT NOT NULL,
    source_code    TEXT NOT NULL,
    thumbnail      BLOB,
    author_id      INTEGER REFERENCES users(id),
    status         TEXT NOT NULL DEFAULT 'pending',
    is_featured    INTEGER NOT NULL DEFAULT 0,
    review_message TEXT,
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at    TEXT
);

CREATE TABLE IF NOT EXISTS sessions (
    id         TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);
