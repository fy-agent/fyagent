> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Grok native review dispositions

- Codex P1: added a bounded native-filename existence guard across active/archived date directories, plus regression. Coordinator also prevents unresolved retries, so directory-fsync failure never grants re-entry. Existing-ID guard makes writer itself refuse republishing that identity.
- Codex P2 #1: not adopted. Persisting a local receipt ID is not a native content effect. create_dir/staging failures before atomic publication remain proven no native content; retry reuses the same recorded ID and the existing-ID guard. Making every local preparation error unresolved would incorrectly disable safe retries.
- P2 #3: ignored assistant text is excluded alongside synthetic text; ignored-only history cannot become a final answer. Regression added.
- P2 #4: all three non-Codex writers now classify failed receipt registration before native invocation as proven no native content effect.
- OpenCode create-only race: under targeted followup; do not claim resolved until final implementation/evidence.
- Common runner output completeness/cap: awaits common owner final integration. Do not claim final passage yet.

## Final implementation closure

- OpenCode no-overwrite race: resolved by official import into temporary native DB and atomic target INSERT-only transaction; real Rust writer/native export/next-request loopback proof in opencode-create-only-report.md.
- Common output completeness: root added bounded per-use output caps and explicit OutputIncomplete for truncation, read error, invalid UTF-8 or missing pipe completion. No partial capture may yield Exited success. Full readback uses 256 MiB; version/status output remains 1 MiB. Final canonical regression pending.
