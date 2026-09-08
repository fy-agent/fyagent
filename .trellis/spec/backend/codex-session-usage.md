# Codex Session Usage Sync Contract

## 1. Scope / Trigger

Read this contract before changing Codex JSONL session import, parent/child
fork replay, deferred retry, or `[CODEX-SYNC]` production logs. Owner:
`src-tauri/src/services/session_usage_codex.rs`.

This is a log-and-retry contract, not a second usage calculator. Token deltas,
`proxy_request_logs` writes, replay-prefix insertion, and child-suffix dedup
stay in this owner. Do not classify pending work by localized Chinese or
English error strings.

## 2. Signatures

```text
sync_codex_usage(db) -> SessionSyncResult

PendingReason =
  MissingParent
  | ParentTimelineNotCaughtUp
  | StableForkGap
  | MalformedTimeline
  | InvariantViolation
  | IoFailure
  | DatabaseFailure

FileWatermark = { modified_nanos, size }
PendingEntry = { child, parent?, reason, parent_id?, consecutive_unchanged }
ReplayCaches = { pending, emitted_fingerprints, ... }
```

`ParentLookupError` maps onto `PendingReason` in code. Retry scheduling state
(`pending`) and diagnostic emission state (`emitted_fingerprints`) are separate
maps. Clearing a retryable pending entry must not clear its fingerprint.

## 3. Contracts

- Expected deferred (`MissingParent`, still-growing `ParentTimelineNotCaughtUp`,
  `StableForkGap`) is not a production `WARN` or per-file `INFO`. Default
  production is silent.
- Debug mode may emit at most one aggregate line per `sync_codex_usage` pass:

  ```text
  [CODEX-SYNC] deferred missing_parent=N catching_up=N stable_gap=N total=N
  ```

  The line is omitted when `total == 0`. Counts must not include user paths,
  rollout IDs, session filenames, or file contents.
- `INFO` sync completion is emitted only when `imported > 0`. Unchanged
  expected deferred must not produce a repeated info line every 60 seconds.
- True `MalformedTimeline` / `InvariantViolation` / `IoFailure` /
  `DatabaseFailure` emit `WARN` or `ERROR` once per fingerprint. Re-emit only
  when reason or child/parent watermark changes.
- Log lines may include the closed reason label and at most one 8-hex
  fingerprint. They must not include full user directories, session filenames,
  rollout IDs, tokens, prompts, or session body.
- Fingerprints bind normalized file identity, reason, and child/parent
  size+mtime watermarks. The hash is internal; only the short hex may appear
  in logs.
- `MissingParent` keeps bounded retry until the parent identity appears.
  `ParentTimelineNotCaughtUp` retries while the parent watermark is still
  changing. Unchanged parent+child with an unmet fork becomes `StableForkGap`
  and is not re-parsed every pass. Any watermark or reason change
  re-evaluates immediately.
- Process restart may re-evaluate; it must not restore a per-file WARN storm
  for unchanged expected deferred.
- Log suppression must not change parse, cursor, transaction, or usage
  semantics: deferred child cursors do not advance; parent catch-up imports
  the child exactly once; replay prefix and child suffix stay deduplicated;
  rebuild and incremental totals agree.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Parent file missing | `MissingParent`; retry; no per-file WARN |
| Parent exists but timeline is before child fork and parent watermark is growing | `ParentTimelineNotCaughtUp`; retry; no per-file WARN |
| Parent and child watermarks are unchanged and fork is still unmet | `StableForkGap`; skip re-parse; no per-file WARN/INFO |
| Parent watermark later catches up | Re-evaluate; import child usage once; cursor advances only after success |
| Same expected deferred set for 120 equivalent 60s passes | 0 per-file WARN; 0 repeated INFO |
| Malformed JSONL / identity / invariant / I/O / DB failure, same fingerprint | One `WARN`/`ERROR`; later identical passes stay quiet |
| Same damaged file grows or reason changes | New fingerprint; may warn again |
| Debug logging enabled with expected deferred | At most one aggregate `DEBUG` per pass |
| Debug logging disabled with expected deferred | No deferred production log |
| Localized `父 rollout … 尚未写到 child fork 时刻` string used as classifier | Contract regression; use `ParentLookupError` |

## 5. Good / Base / Bad Cases

- Good: a child fork waits for the parent timeline, stays silent in production,
  then imports exactly once when the parent file grows past the cutoff.
- Base: a pass that imported nothing and only holds expected deferred prints
  no `INFO`.
- Bad: `mark_deferred` `warn!` per child path; deleting retryable pending so
  the next pass looks new; `INFO` whenever `deferred_files > 0`; globally
  silencing the module; advancing the child cursor to hide the lag.

## 6. Tests Required

Covered in `session_usage_codex` tests:

- 120 unchanged expected-fork passes: zero per-file WARN, zero repeated INFO;
- parent catch-up recovers usage exactly once;
- `StableForkGap` skips re-parse until a watermark change;
- fingerprint change re-evaluates;
- true corruption dedup and restart first pass without a WARN storm;
- debug aggregate at most one line per pass;
- existing missing-parent recovery, replay prefix, and duplicate-insert
  invariants remain green.

`mise run check:backend` is the owning gate. Do not treat CI silence as
native Windows HIL of a user-provided rollout corpus.

## 7. Wrong vs Correct

#### Wrong

```rust
pending.remove(path); // also forgets that this WARN already fired
mark_deferred(path, "父 rollout 尚未写到 child fork 时刻");
log::warn!("[CODEX-SYNC] deferred {}: {reason}", path.display());
if result.deferred_files > 0 {
    log::info!("[CODEX-SYNC] 同步完成: ... deferred {}", result.deferred_files);
}
```

#### Correct

```rust
// retry scheduling and emitted_fingerprints are independent
if reason.is_expected_deferred() {
    // production: silent; debug: one aggregate count line
} else if fingerprints.insert(short_hex) {
    log::warn!("[CODEX-SYNC] deferred reason={} fingerprint={short_hex}", reason.as_label());
}
if imported > 0 {
    log::info!("[CODEX-SYNC] 同步完成: 导入 {imported} 条, ...");
}
```
