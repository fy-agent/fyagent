> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Native writer rework — 2026-09-22

Owner: native_continuation. Product scope: only `migrate/native/opencode.rs`,
`hermes.rs`, `gemini.rs`. No commits, provider installation, real account calls,
or Windows execution were performed by this work package.

## Implemented behavior

All three writers reuse the shared selected CLI, the current store instance
identity and any `target_native_id` already on the receipt. They record an ID
before content publication, reject an existing ID, and leave an uncertain
native call unresolved. They use the shared formal Windows execution guard.
Request JSON travels through private temporary files; transcript text is never
assembled into a command or executable script. All temporary bridge programs
are fixed literals in the product modules. Errors do not expose provider
configuration, native stderr, or credentials.

| Provider | Exact gate | Native write | Native readback |
| --- | --- | --- | --- |
| OpenCode | 1.18.30 | official `import --pure` | official `export --pure` |
| Hermes | 0.20.5 SDK | installed `SessionDB.create_session` and `append_message` | official `sessions export - --session-id ID --format jsonl` |
| Gemini | 0.46.0 active npm bundle | native JSONL layout with preallocated UUID and exclusive file creation | installed `loadConversationRecord` and `convertSessionToClientHistory` |

OpenCode uses `debug config --pure` in the selected target workspace to resolve
the target model. Only `provider/id` is retained; the whole configuration is
never exposed or copied. `info.model` is set to that target model, so a bare
resume uses the target installation without an extra model flag. Historical
messages use the same target model. Native timestamps increase with `seq`;
source timestamps cannot reorder the transcript. Assistant records have the
required `agent`, `finish=stop` and `time.completed`, and the header has the
actual native producer version 1.18.30. A leading assistant without a user
parent is rejected before recording an ID.

Hermes foreign import was removed because it trims and merges content. Before
creating a database session, the fixed SDK bridge invokes the installed native
decoder for both display and model replay and compares ordered `(role,text)`
values exactly. It also checks the decoder after appending a synthetic next
user row, solely in memory: the synthetic row is never stored. This rejects
leading/trailing whitespace that Hermes strips, consecutive messages it
merges, and a trailing unanswered user that would merge with the next prompt.
Internal CRLF, Unicode and paths are preserved when the native decoder can
represent them. Imported finals receive `finish_reason=stop`; no source model,
model configuration, `api_content`, display override or authentication data
is copied. Raw Hermes records have no producer-version field; this writer does
not fabricate one.

Gemini imports only the selected installation's active, version-checked bundle
exports. It does not invoke CLI main, auth, settings, summaries or model calls.
`--list-sessions` is deliberately not used: the installed implementation can
generate summaries through a model. Before publication, the native history
converter must preserve every role and text exactly; user text beginning with
native command/context prefixes is rejected. New files are created with `wx`
and mode 0600, flushed, and read back through the official loader. A partial
write remains unresolved. The selected Gemini directory must match the tested
`GEMINI_CLI_HOME/.gemini` layout.

## Native evidence actually achieved

All probes ran on macOS using newly created synthetic HOME, XDG, HERMES_HOME
and GEMINI_CLI_HOME directories under `work/native-writer-staging`. The child
environment was rebuilt from only PATH/LANG plus fixture variables. Proxy
variables blocked non-loopback routes; the only model endpoint was an in-process
127.0.0.1 HTTP server with a fake key. It returned HTTP 400 after capture, with
no generated answer. The installed providers were not upgraded.

The reproducible probe is `work/native-writer-staging/verify_native_writers.py`.
The final native run is `work/native-writer-staging/run-6013a1ceaf`. Selected
evidence is in `evidence/native-continuation/writer-rework/`.

| Provider | Native write/read | Next-turn request proof | Important limit |
| --- | --- | --- | --- |
| OpenCode | import exit 0; exact five-message native export | bare `run --session` sent all five original messages, in order, to loopback `local` | Probe fixture matches the Rust encoding schema; it is not the Rust orchestrator executing the real CLI. Rust encoder and orchestrator have separate production-module tests. |
| Hermes | exact product `SDK_BRIDGE` created four messages; official export preserved all bodies and final markers; repeat ID returned `alreadyExists` | `/v1/responses` contained all four original messages followed by the new user message | Whitespace trimming, merged adjacent turns and unanswered trailing users are rejected before any session write. |
| Gemini | exact product `NATIVE_BRIDGE` created five messages; official loader/converter read them exactly in fresh processes; repeat ID returned `alreadyExists` | streamGenerateContent contained all five original messages separately, including consecutive roles, CRLF, outer whitespace and trailing unanswered user | Target runtime added its own separate context; model alias resolved to `gemini-3.5-flash`. No source model/account was imported. |

The retained `hermes-unanswered-counterexample.json` is explicitly superseded:
it records the pre-fix native behavior that merged a trailing unanswered user
with the next prompt. The final run confirms this shape now returns
`bodyNotPreserved` before publication.

This proves installed native write/read mechanisms and request construction.
It does not prove a real model response, desktop UI acceptance, or Windows
execution. The user waived unavailable Windows real-machine acceptance; the
formal elevated Windows security guard remains in force.

## Final-only export boundary

OpenCode output carries actual native final markers and is tested through the
strict extraction entry point. Hermes official export confirms the SDK wrote
`finish_reason=stop`, with unmodified content and no API/display injection; the
strict Hermes SQLite reader recognizes that native shape.

Gemini 0.46.0 does **not** durably record a reliable final-completion signal.
Previous installed-source probes proved that a missing finishReason can leave
the same persisted message shape as STOP. This writer accepts an already
validated final-only package and verifies its known projection, but does not
make arbitrary raw Gemini history exportable. No invented final marker is
inserted into user content. Receipt provenance and any future safe re-export
policy remain coordinator-owned. The capability matrix must keep native
restore support separate from raw final-only extraction support.

## Rust orchestration validation

Focused module tests exercise the actual production writer entry points against
isolated fake executables. They verify version rejection, store mismatch,
receipt failure without content publication, receipt-before-content ordering,
preallocated/reused ID preservation and refusal to overwrite an existing ID.
These are fault-injected orchestration tests, not real-provider end-to-end runs.
Pure projection tests cover body/order/role/digest sensitivity and required
final markers; OpenCode also re-exports its actual encoded projection through
the strict extractor.

The first canonical `mise run rust:test -- session_manager::migrate::native`
run passed **19 tests**, including OpenCode's full Rust restore orchestration
test and all three providers' earlier projection tests. The final run after
adding the Hermes/Gemini orchestration tests and OpenCode strict re-export
assertion encountered an unrelated shared-file edit during compilation:
`model.rs:846` called the removed `sanitize_stderr_tail` function (E0425).
No error was reported in the three native writer modules, but the newly added
assertions did **not** run. The coordinator requested no further overlapping
whole-crate compile and will perform the final combined verification after the
shared writer finishes. `rust-native-tests.log` records this compile blocker;
it must not be presented as a passing final test run. All three owned modules
passed rustfmt parsing after their final edit.

## Integration requirements

- `native/mod.rs` owns registration, `provider_cli_path`, process/output bounds
  and the formal Windows guard. No alternate process framework was added.
- `NativeRestoreInput.target_native_id` must contain the previous receipt's ID
  on a proven-safe retry. The writer never silently replaces a supplied ID.
- Store and receipt identity inheritance remain coordinator-owned.
- Exact version/layout mismatches and native text-shape limitations fail
  closed. The unrelated installed npm-global Gemini 0.59.0 is not covered by
  the Homebrew 0.46.0 evidence.
- Per-provider readback output can be larger than generic version-probe
  output. The shared runner's bounded output policy must allow the product's
  admitted package sizes or reject oversized requests before publication;
  truncation must never count as verified readback.
