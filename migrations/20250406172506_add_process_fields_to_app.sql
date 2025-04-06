-- Migration file: 20240406000001_process_management.sql

-- Add new columns to apps table
ALTER TABLE apps ADD COLUMN restart_policy TEXT NOT NULL DEFAULT 'on-failure';
ALTER TABLE apps ADD COLUMN max_restarts INTEGER NULL;
ALTER TABLE apps ADD COLUMN restart_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE apps ADD COLUMN last_exit_code INTEGER NULL;
ALTER TABLE apps ADD COLUMN last_exit_time TIMESTAMP NULL;
ALTER TABLE apps ADD COLUMN startup_timeout INTEGER NOT NULL DEFAULT 30;
ALTER TABLE apps ADD COLUMN shutdown_timeout INTEGER NOT NULL DEFAULT 10;
ALTER TABLE apps ADD COLUMN health_check TEXT NULL;

-- Create a table for process history
CREATE TABLE IF NOT EXISTS process_history (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP NULL,
    exit_code INTEGER NULL,
    exit_reason TEXT NULL,
    FOREIGN KEY (app_id) REFERENCES apps(id)
);

-- Create index on app_id for quick lookups
CREATE INDEX IF NOT EXISTS idx_process_history_app_id ON process_history(app_id);
