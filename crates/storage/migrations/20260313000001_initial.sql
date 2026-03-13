CREATE TABLE media_cache (
    file_path   TEXT PRIMARY KEY,
    file_size   INTEGER NOT NULL,
    modified_at TEXT NOT NULL,
    media_info  TEXT NOT NULL,
    probed_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    project_json TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    modified_at TEXT NOT NULL
);
