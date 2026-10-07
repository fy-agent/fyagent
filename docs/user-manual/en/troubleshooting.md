# Troubleshooting and file locations

| Situation                                    | Next step                                                                                                   |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Software not found or installation uncertain | Scan again in 「AI软件配置」; finish any installation wizard, refresh and select the intended installation. |
| Login remains unconfirmed                    | Finish the official page and refresh the account or software authentication status.                         |
| Model request fails                          | Check URL, API Key, model ID, network and current model source; use 「测试连通」 in model management.       |
| Saved settings have no effect                | Reload current settings, then restart the target software or start a new session as instructed.             |
| Settings changed elsewhere                   | Refresh and inspect them before confirming again; retain any draft you need.                                |
| Assigned Skill / MCP unavailable             | Check target switches, directories, runtimes and credentials, then reload the target software.              |
| Previous results remain visible              | Retry or refresh and inspect current content before writing again.                                          |

Legacy WebDAV/S3 cloud sync is retired. Existing user configuration files are not deleted or modified; FyAgent no longer reads/uses them or performs cloud sync.

## Default locations

`~` is your user home directory. Custom directories and environment variables can change actual paths; prefer the operation dialog and the software's configuration.

| Content                                        | Default path                                                       |
| ---------------------------------------------- | ------------------------------------------------------------------ |
| FyAgent database and device settings           | `~/.fyagent/fyagent.db`, `~/.fyagent/settings.json`                |
| FyAgent Skills and backups                     | `~/.fyagent/skills/`, `~/.fyagent/skill-backups/`                  |
| Claude Code configuration and prompt           | `~/.claude/settings.json`, `~/.claude/CLAUDE.md`                   |
| Claude MCP                                     | `~/.claude.json`                                                   |
| Codex authentication, configuration and prompt | `~/.codex/auth.json`, `~/.codex/config.toml`, `~/.codex/AGENTS.md` |
| Gemini prompt                                  | `~/.gemini/GEMINI.md`                                              |
| OpenCode configuration and prompt              | `~/.config/opencode/opencode.json`, `~/.config/opencode/AGENTS.md` |
| OpenClaw workspace                             | `~/.openclaw/workspace/`                                           |
| Hermes long-term memory                        | `~/.hermes/memories/MEMORY.md`, `~/.hermes/memories/USER.md`       |

Back up files before editing them directly and avoid concurrent writes with FyAgent. Reload the page afterward. If environment variables conflict with authentication or service settings, inspect the target software's actual source and retain needed values before adjusting them.

For help, include the OS, FyAgent version, software name, steps and redacted error in a report following [Support](../../../SUPPORT.md). Keep API Keys, tokens and authentication file contents out of screenshots and public issues.

## Local database recovery

Open 「关于 FyAgent → 本机备份与恢复」, select an existing `.db` copy and confirm. This replaces the local database after establishing a recovery point. The selected copy remains; credentials retain device-binding limits.

- Database not replaced: retain the selected backup and address validation/preparation/disk-permission failures; retry only when offered.
- Database replaced but checking failed: record 「所选副本」 and 「恢复前备份」 filenames from the result, preserve the files in the default `~/.fyagent/backups/`, and use 「检查当前数据库」. Do not restore again blindly.
- Replacement unknown: retain database/backups and check the current database first. 「刷新备份」 rereads a list only; it cannot confirm restored content. Closing does not cancel a started restore.

「检查当前数据库」 only checks whether the current database is readable; it cannot confirm that its contents match the selected copy. After completion, reopen relevant pages and check accounts, models and history. Portable connection configuration excludes API Keys, account bindings and device-specific settings.

[Back to manual](README.md)
