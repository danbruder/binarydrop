-- migrations/20250403_initial.sql
-- Create apps table
CREATE TABLE IF NOT EXISTS apps (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    state TEXT NOT NULL,
    binary_path TEXT,
    binary_hash TEXT,
    port INTEGER NOT NULL,
    environment TEXT NOT NULL,
    process_id INTEGER,
    host TEXT NOT NULL
);

-- Create index on app name
CREATE INDEX IF NOT EXISTS idx_apps_name ON apps(name);
