CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL CHECK(length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,
    elapsed_ms INTEGER NOT NULL CHECK(elapsed_ms >= 0),
    open INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX sessions_period ON sessions(started_at, ended_at);
CREATE INDEX sessions_end ON sessions(ended_at);
CREATE TABLE preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
PRAGMA user_version = 1;
