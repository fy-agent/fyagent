# Strict native extraction implementation

2026-09-22. Owner: native strict_extraction implementer. Production scope is only `src-tauri/src/session_manager/migrate/extract/**`. No source sessions, credentials, installed CLI files or remote services were mutated. Tests use generated temporary SQLite and JSON fixtures.

## Delivered behavior

- `extract_session_with_source` returns the projected session and metadata from one source read. `RawSource.source_cli_version` records only native source metadata; `canonical_store_path` identifies the source's actual native store for the coordinator's exact receipt mapping.
- Codex uses `session_meta.cli_version` and requires one header. Installing 0.154.0 cannot certify an older or unversioned rollout. Known `phase=final_answer` events without matching lossless response items block, counting repeated identical answers separately; legacy unphased events are not guessed. Mixed `user.text` and runtime provenance, malformed kinds, missing payload and unknown response-item types block the whole extraction. Pure runtime messages remain omitted. Arbitrary copied rollouts are not bound to the current Codex installation.
- OpenCode 1.18.30 now loads its native SQLite and official JSON export. SQLite metadata/messages/parts are read in one query-only snapshot with bounded bytes and record counts. Native message ordering is `(time_created,id)`; part ordering is `id`, matching official MessageV2. User synthetic/ignored parts are excluded individually. Explicit stop, completed persistence, error absence, closed tools and step boundaries establish finals; a tool-calls step's narration is excluded. The last step's finish must match the message finish. An unfinished later step cannot reuse an earlier stop.
- Hermes 0.20.5 now loads native SQLite without calling the lossy display decoder. It requires all relevant provenance columns, reads only active noncompacted rows in native insertion order, decodes the exact NUL-json content prefix, never reads API sidecars, and validates tool call/result closure. Every exported assistant has explicit stop and no tool call. Structured display_kind injections are excluded; untyped runtime marker collisions block rather than silently deleting literal user text.
- Shared text projection no longer invents newline separators. Providers reject multiple independently authored text boundaries where flattening is not proven. Original whitespace, CRLF, Unicode and paths remain unchanged.
- Source JSON uses the existing duplicate-key rejection function. Read-only SQLite rows and JSON/JSONL have cumulative bounds; every malformed or unclassified final blocks the entire session, even in the middle.

## Explicit capability limits

These are per-format evidence limits, not permanent provider scope changes:

- Hermes does not persist a CLI producer version. `origin.cliVersion` stays absent. The current 0.20.5 gate establishes parser/schema verification only; legacy rows without finish/provenance, JSONL without required columns, compaction/child lineages, or stripped provenance markers are rejected.
- Legacy OpenCode directory layouts are not assigned the SQLite 1.18.30 contract; native revert projections need separate verification. Multiple user/final text-part boundaries with no proven lossless flattening are blocked.
- Gemini 0.46.0 remains blocked because installed code records the same native assistant shape before checking whether finishReason exists. The capability gap now points to this actual counterexample. Claude, OpenClaw and Grok extraction gaps remain explicit.
- Raw source exports outside the current native store never gain a local receipt merely because a native ID matches. Root export owns the final current-device/store/native-ID lookup.

## Writer coordination

Native worker confirmed OpenCode output includes version 1.18.30, assistant stop/completed, one unchanged text part and real target-local model. Hermes SDK output includes assistant finish_reason=stop and empty display/API metadata. Static cross-check found a native OpenCode ordering bug: source timestamps could reorder transcript rows; native worker fixed publication to target now+seq and rejects a leading assistant before mutation.

## Source evidence

- Existing `extraction-source-evidence.md`, `native-continuation-report.md`, and `gemini-native-report.md`.
- Exact [OpenCode v1.18.30 processor](https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/session/processor.ts): step finish establishes finish reason; cleanup also completes aborted records. `message-v2.ts` fixes message/part order.
- Installed Hermes 0.20.5 `hermes_state.py` append/decode/read functions and `agent/context_compressor.py` provenance rules. The latter explicitly documents dropped underscore metadata; no arbitrary natural-language guessing is used to discard input.

## Validation status

- Rustfmt syntax/format check on extraction module only: passed.
- First canonical `mise run rust:test session_manager::migrate::extract`: stopped at unrelated in-progress cross-package compiler errors (export/commands/native input fields). No extraction compiler error was emitted, but no extraction test ran. Coordinator requested no repeat until interface publication.
- Final substantive-code canonical focused run: `mise run rust:test session_manager::migrate::extract` passed **45 tests / 0 failures**, process exit 0. This ran the actual production crate's module tests; separate integration test targets were filtered by the requested name, not claimed as executed.
- After that passing run, coordinator requested the compatibility-only `extract_session` wrapper be gated with `#[cfg(any(test, feature = "test-hooks"))]`; applied. Test configuration and function body are unchanged. Root will perform the final whole-repository check/build/test after all owners integrate.
- Remaining compiler warnings belong to shared native/receipt/dialog/model work in progress; no extraction code warning remains in ordinary production builds after the wrapper gate.
- Writer lock released after delivery. Root now owns integration, final validation and PR inclusion.

Module test inventory:
- `src-tauri/src/session_manager/migrate/extract/strict_text.rs`: 7 tests.
- `src-tauri/src/session_manager/migrate/extract/mod.rs`: 5 tests.
- `src-tauri/src/session_manager/migrate/extract/storage.rs`: 0 tests.
- `src-tauri/src/session_manager/migrate/extract/rules/opencode.rs`: 8 tests.
- `src-tauri/src/session_manager/migrate/extract/rules/hermes.rs`: 8 tests.
- `src-tauri/src/session_manager/migrate/extract/rules/codex.rs`: 14 tests.
- `src-tauri/src/session_manager/migrate/extract/rules/mod.rs`: 3 tests.
