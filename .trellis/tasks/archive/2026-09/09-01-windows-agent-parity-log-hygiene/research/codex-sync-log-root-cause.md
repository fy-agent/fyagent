# Codex sync deferred log root cause

## User-visible symptom

Windows produces long repeated lines equivalent to:

```text
[WARN][fyagent_lib::services::session_usage_codex] [CODEX-SYNC] deferred <child>: 父 rollout <parent> 尚未写到 child fork 时刻
```

The same child/parent pair can be logged repeatedly during periodic synchronization.

## Relevant current flow

Owner: `src-tauri/src/services/session_usage_codex.rs`.

### 1. Replay-prefix decision

When a child rollout contains fork metadata, the synchronizer asks the parent timeline for signatures before the child fork cutoff. If the current parent maximum timestamp is still earlier than that cutoff, `signatures_before` returns a retryable error whose semantic meaning is “parent has not been written far enough yet”.

This is expected in an append-only producer when parent and child files become visible at slightly different times.

### 2. Deferred cache

`mark_deferred` writes a pending entry and intends to warn only when the previous pending entry is different.

That would be a reasonable transition-based rule if the previous state were retained.

### 3. Dedup state is removed before retry

At the beginning of a later `sync_single_codex_file` pass, an unchanged pending entry with `Retryable(_)` is removed so the file can be retried.

When the parent is still behind, `mark_deferred` inserts the same state again. Because the old pending entry was removed, it appears new and the WARN is emitted again.

Therefore the current code couples two independent concerns:

- whether work should be retried;
- whether the same diagnostic has already been emitted.

Clearing retry state also clears diagnostic memory.

### 4. Per-pass INFO amplification

The overall sync summary emits INFO whenever deferred files exist. An unchanged expected deferred set therefore produces another line every periodic pass even if per-file WARN were fixed.

## Correctness constraints

A log-only suppression patch is insufficient. The implementation must preserve:

1. unchanged child files are retried when parent identity/stamp/timeline changes;
2. deferred child cursor is not advanced;
3. parent replay prefix is inserted exactly once when available;
4. child suffix remains deduplicated against parent signatures;
5. rebuild and incremental totals agree;
6. malformed JSONL/identity/database failures remain visible.

## Target semantics

### State model

Keep retry scheduling and diagnostic emission state separately, or retain a stable diagnostic fingerprint while removing only the computed retry result.

A minimal semantic state includes:

```text
child stable identity/stamp
parent stable identity/stamp when known
bounded reason class
first/last seen
consecutive observations or age threshold
last emitted fingerprint/severity
```

Exact data structures may be smaller and should fit the existing cache/persistence owner.

### Emission policy

| Transition | Expected output |
| --- | --- |
| first normal parent-write lag | debug, or one bounded aggregate info |
| same lag on later passes | no warn/info |
| reason/parent identity/relevant stamp changes | at most one new bounded event |
| lag exceeds a reviewed abnormal threshold | one warn/aggregate escalation, not every pass |
| recovery | one bounded debug/info recovery event |
| malformed fork identity/JSONL/DB failure | warn/error according to existing severity |

### Privacy

Routine events must not contain the complete Windows user path or raw rollout filename. A short stable fingerprint/count/reason class is sufficient. Full paths may remain in explicitly gated developer diagnostics only if current privacy spec permits it.

## Required tests

1. same child stamp + same parent-behind state across N passes -> zero repeated WARN/INFO after the initial allowed event;
2. child unchanged + parent stamp increases but still behind -> retry occurs; diagnostic emission remains bounded;
3. parent reaches cutoff -> child recovers, usage appears exactly once, one recovery event at most;
4. parent identity changes -> state is recomputed and one changed diagnostic may emit;
5. malformed parent/child metadata -> real warning remains;
6. deferred child cursor remains unchanged;
7. rebuild and incremental totals match;
8. no test depends on global logger timing when a pure emission-policy helper/test sink can be used.

## Non-solution examples

- globally lowering all `session_usage_codex` logs;
- deleting the WARN line while retaining per-pass INFO spam;
- marking the file terminal/non-retryable;
- advancing the child cursor to silence retries;
- applying a fixed sleep before every sync;
- hiding logs only on Windows;
- matching the Chinese error string rather than a closed retry reason class.
