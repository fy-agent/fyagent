-- Original FyAgent v26 DDL for the tables extended by v27.
CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            directory TEXT NOT NULL,
            repo_owner TEXT,
            repo_name TEXT,
            repo_branch TEXT DEFAULT 'main',
            readme_url TEXT,
            enabled_claude BOOLEAN NOT NULL DEFAULT 0,
            enabled_codex BOOLEAN NOT NULL DEFAULT 0,
            enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
            enabled_grokbuild BOOLEAN NOT NULL DEFAULT 0,
            enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
            enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
            enabled_qoderwork BOOLEAN NOT NULL DEFAULT 0,
            enabled_trae_work BOOLEAN NOT NULL DEFAULT 0,
            enabled_workbuddy BOOLEAN NOT NULL DEFAULT 0,
            installed_at INTEGER NOT NULL DEFAULT 0,
            content_hash TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0
        );

CREATE TABLE IF NOT EXISTS session_log_sync (
                file_path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL,
                last_line_offset INTEGER NOT NULL DEFAULT 0,
                last_synced_at INTEGER NOT NULL
            );

INSERT INTO skills (id, name, directory, enabled_codex, enabled_workbuddy)
VALUES ('kept-skill', 'Kept skill', 'kept-skill', 1, 1);
INSERT INTO session_log_sync VALUES ('/tmp/a.jsonl', 5, 3, 1);
PRAGMA user_version = 26;
