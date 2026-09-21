# 历史文本（工作站路径已匿名化）：evidence/native-continuation/hermes-export-help.txt

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/native-continuation/hermes-export-help.txt` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：5441
- 原稿 SHA-256：`7cd1cc09a391e27e577ad461c6d2de9622c1bbeb579e942077359ac1ddceb254`
- 匿名化载荷字节数：5441
- 匿名化载荷 SHA-256：`7cd1cc09a391e27e577ad461c6d2de9622c1bbeb579e942077359ac1ddceb254`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```text
usage: hermes sessions export [-h] [--format {jsonl,md,qmd,html,trace}]
                              [--upload] [--public] [--no-redact]
                              [--only {user-prompts}]
                              [--session-id SESSION_ID] [--older-than AGE]
                              [--newer-than AGE] [--before TIME]
                              [--after TIME] [--source SOURCE] [--title TITLE]
                              [--end-reason END_REASON] [--cwd CWD]
                              [--min-messages MIN_MESSAGES]
                              [--max-messages MAX_MESSAGES] [--model MODEL]
                              [--provider PROVIDER] [--user USER]
                              [--chat-id CHAT_ID] [--chat-type CHAT_TYPE]
                              [--branch BRANCH] [--min-tokens MIN_TOKENS]
                              [--max-tokens MAX_TOKENS] [--min-cost MIN_COST]
                              [--max-cost MAX_COST]
                              [--min-tool-calls MIN_TOOL_CALLS]
                              [--max-tool-calls MAX_TOOL_CALLS] [--dry-run]
                              [--yes] [--redact] [--lineage {single,logical}]
                              [--delete-after-verified] [--force]
                              [output]

positional arguments:
  output                Output path. JSONL: file path (use - for stdout,
                        required). md/qmd: output directory (default: <hermes
                        home>/session-exports)

options:
  -h, --help            show this help message and exit
  --format {jsonl,md,qmd,html,trace}
                        Export format (default: jsonl). 'trace' emits Claude
                        Code JSONL for the Hugging Face Agent Trace Viewer
  --upload              trace only: upload to your Hugging Face traces dataset
                        instead of writing a local file (needs HF_TOKEN)
  --public              trace --upload only: create/update a public dataset
                        instead of private
  --no-redact           trace only: skip the forced secret redaction; only use
                        after manual review
  --only {user-prompts}
                        Export only a filtered view (user-prompts: one prompt
                        record per line for jsonl, headed sections for md)
  --session-id SESSION_ID
                        Session ID or unique prefix to export
  --older-than AGE      Only export sessions older than AGE (duration like
                        '5h'/'2d', bare number of days, or an ISO timestamp)
  --newer-than AGE      Only match sessions active within the last AGE (e.g.
                        '5h', '2d') or after an ISO timestamp
  --before TIME         Only match sessions started before TIME (duration ago
                        like '5h', or ISO timestamp like '2026-07-05 14:30')
  --after TIME          Only match sessions started at/after TIME (duration
                        ago like '5h', or ISO timestamp)
  --source SOURCE       Only match sessions from this source
  --title TITLE         Only match sessions whose title contains this
                        substring
  --end-reason END_REASON
                        Only match sessions with this end reason
  --cwd CWD             Only match sessions whose working directory is under
                        this path
  --min-messages MIN_MESSAGES
                        Only match sessions with >= N messages
  --max-messages MAX_MESSAGES
                        Only match sessions with <= N messages
  --model MODEL         Only match sessions whose model name contains this
                        substring (e.g. 'sonnet', 'gpt-5', 'hermes')
  --provider PROVIDER   Only match sessions billed through this provider (e.g.
                        openrouter, anthropic, nous)
  --user USER           Only match sessions from this user ID
  --chat-id CHAT_ID     Only match sessions from this chat/channel ID
  --chat-type CHAT_TYPE
                        Only match sessions with this chat type (e.g. dm,
                        group)
  --branch BRANCH       Only match sessions whose git branch contains this
                        substring
  --min-tokens MIN_TOKENS
                        Only match sessions with >= N total tokens
                        (input+output)
  --max-tokens MAX_TOKENS
                        Only match sessions with <= N total tokens
                        (input+output)
  --min-cost MIN_COST   Only match sessions costing >= N USD (actual or
                        estimated)
  --max-cost MAX_COST   Only match sessions costing <= N USD (actual or
                        estimated)
  --min-tool-calls MIN_TOOL_CALLS
                        Only match sessions with >= N tool calls
  --max-tool-calls MAX_TOOL_CALLS
                        Only match sessions with <= N tool calls
  --dry-run             List matching sessions without changing anything
  --yes, -y             Skip confirmation
  --redact              Redact secrets (API keys, tokens, credentials) from
                        exported content
  --lineage {single,logical}
                        md/qmd only: export one row or its compression lineage
  --delete-after-verified
                        md/qmd only: after verified single-session export,
                        delete that session (needs --yes)
  --force               md/qmd only: overwrite an existing export file
```
