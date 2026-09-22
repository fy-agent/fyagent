> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Gemini native verification — exact installed 0.46.0

2026-09-22. Read-only production verification by the native-continuation executor. No production files, installed CLI files or real user session/config/credential files were modified. Synthetic storage is under `work/gemini-continuation`. No real provider inference, installation or upgrade occurred. Model traffic was sent only to a loopback mock, which returned HTTP 400 deliberately before inference.

## Verdict

**The native import and continuation path is real, but this does not establish a final-only source extractor.**

- Installed CLI `--session-file` successfully imports a minimal synthetic conversation into a fresh native session ID.
- Official `--list-sessions` shows that ID; a separate restarted CLI shows it again.
- Separate processes executing the installed CLI's own exported `resolveSessionId` / `loadConversationRecord` functions read back every original message exactly, before and after the attempted next turn.
- `--resume <new ID> --prompt ...` sends all five original messages to the local HTTP mock in the same order and as separate roles/parts. The fixture covers consecutive users, consecutive AI answers, an unanswered trailing user, Chinese, a Windows path, CRLF and surrounding whitespace. All bodies match exactly.
- Native resume silently filters certain user text prefixes. More critically, raw native Gemini records do **not** carry a dependable final-answer completion signal. A persisted `type=gemini` record with no tool calls is insufficient. Current ordinary native history must remain `finalAnswerIndeterminate` unless additional trustworthy turn-completion evidence exists.

## Entry and version discrepancy resolved

Two installations coexist:

| Entry | Actual isolated `--version` | Used for this probe |
| --- | --- | --- |
| `/opt/homebrew/bin/gemini` | **0.46.0** | Yes, fixed absolute path |
| `/Users/<username>/.npm-global/bin/gemini` | **0.59.0** | Only isolated version/help discovery |

Homebrew real package: `/opt/homebrew/Cellar/gemini-cli/0.46.0/libexec/lib/node_modules/@google/gemini-cli`.

Its `bundle/gemini.js:118` imports `gemini-YXO2QQ66.js`; that active module imports `chunk-RCJSF5RP.js`. The package contains other platform/build chunks, which were **not** used to establish this version's behavior. SHA-256 hashes and package version are recorded in `../evidence/gemini-native/source-manifest.json`. Future capability checks must bind the CLI resolved path and version together; neither the PATH name nor the other installation proves compatibility.

The prior report's `main`-branch format description is superseded for these claims by the exact installed bundle and local execution evidence.

## Official import, record and read paths

Active `gemini-YXO2QQ66.js:15816–15885` implements `resolveSessionId`:

1. Calls installed `loadConversationRecord` on the supplied JSON/JSONL.
2. Keeps only object messages with `type=user|gemini` and a defined `content`.
3. Adds an informational import-path message.
4. Generates a new session UUID and sets target `projectHash`, `startTime` and `lastUpdated`.
5. Writes native JSONL to the target project's `chats` directory.

The input used only `sessionId`, `projectHash`, timestamps, optional safe summary, and messages containing unique id/timestamp/type plus exactly one `{text}` content part. It contained no source model, account, tools, thoughts, instructions, memory scratchpad or directories. The CLI chooses the model from the **target** invocation/config. This probe requested local target `gemini-2.5-flash`; the installed runtime routed the wire request to `gemini-3.5-flash`. The wire model is therefore reported separately rather than claiming an exact model solely from the startup label.

`GEMINI_CLI_HOME` is a **home root**, not the `.gemini` directory: `chunk-RCJSF5RP.js:248865–248871` returns it from the internal home resolver, and storage adds `.gemini`. The isolated store here was `work/gemini-continuation/gemini/.gemini/...`. `HOME`, XDG locations and temporary output were also isolated; environment variables carrying real provider credentials were not inherited.

The native loader at `chunk-RCJSF5RP.js:281671–281853` applies `$rewindTo`, `$set`, repeated message IDs and legacy JSON fallback. A source reader must use these semantics or an equivalent tested projection; scanning every JSONL row as a distinct message is incorrect.

There is no public CLI full-transcript export tested here. The exact installed bundle **does export** `loadConversationRecord`, and the CLI module exports `resolveSessionId`; the helper imports those unchanged functions in a fresh Node process. This is an official implementation readback, stronger than a FyAgent file parser, but its module/chunk names are private packaging details and must not be hardcoded as a stable cross-version API. CLI listing establishes native discoverability; captured next-turn requests establish actual model-context loading. No interactive screen or real model response was tested.

## Two blocking counterexamples for a broad capability claim

### User prefix filter

`convertSessionToClientHistory`, active core bundle `327751–327810`, skips a user message when its trimmed text starts with `/`, `?`, `<session_context>` or `<hook_context>`. Direct calls to the installed function verified that literal user strings with each prefix produce an empty history; an ordinary user string is retained.

Thus a native write and successful readback alone cannot claim exact next-turn context for **every** arbitrary user text. Preserve the package body. Do not escape or prepend synthetic characters to defeat this filter. A writer using this version needs a preflight rejection/reason for affected sessions or additional upstream support. This is an exact-version native limitation, not a permanent Gemini product restriction.

### Final answer cannot be inferred from the durable message record

`GeminiChat.processStreamResponse`, active core bundle `328487–328649`, accumulates streaming parts and separately tracks `finishReason`. It then calls `ChatRecordingService.recordMessage` **before** checking that a finish reason exists. The service at `282003–282024` persists id/timestamp/type/content, thoughts, tokens and model; it does not persist `finishReason` or a turn-final phase.

A controlled direct execution of those **installed, unchanged methods** proved the ambiguity:

| Synthetic input to official stream processor | Native record | Result |
| --- | --- | --- |
| Text `SYNTHETIC_FINAL_LIKE`, no finish reason | `type=gemini`, same content, thoughts=[], tokens=null, model set | Throws `InvalidStreamError: Model stream ended without a finish reason.` **after recording** |
| Same text with finishReason=STOP | Structurally identical native record, apart from generated id/timestamp | Completes normally |

Therefore `no toolCalls`, nonempty content, tokens, model, a later user message or being the last assistant message does not supply the missing positive completion proof. The source writer also trims accumulated response text before this record, so native history cannot recover bytes it never saved.

The counterexample is in `finish-result.json`; it uses no API or model. The product must **not** guess from natural-language tone or last-assistant position. Gemini source export remains indeterminate without reliable complementary completion evidence. A known-valid FyAgent package can still be a candidate for native restoration when its origin/provider policy permits it, but native import success must not be used to launder uncertain source answers into verified finals.

## Minimal implementation guidance

- Use only target-local working directory and model/auth config. The minimal imported records need no model field.
- Set a safe session summary/title when available, but never copy arbitrary source metadata such as `memoryScratchpad`, directories or configuration.
- The native importer mints the target UUID. Persist a pending receipt before invoking it; a lost response needs deterministic reconciliation rather than blind reimport. Generated unique message IDs survive import and can carry a non-body identity marker, subject to exact-version verification. Do not search by text alone.
- **Do not use `--list-sessions` as a universally read-only writer terminator.** Active `gemini-YXO2QQ66.js:15471` first calls `generateSummary`; the experiment observed a separate summary model request for an older synthetic session lacking a summary. In production this could involve real target history and inference. The actual read-only/list evidence here used the local mock only.
- `--session-file <file> --list-extensions` was also tested against the isolated target with a closed loopback model endpoint: exit 0 and a new native file/session were produced without inference. Its source branch exits before `listSessions` and headless prompting. This avoids the known summary call, but still initializes target CLI/config/auth; process exit does not by itself return a structured target UUID. Do not bypass receipt reconciliation.
- A narrow version-gated bridge to the installed exported `resolveSessionId` can return the UUID/path as JSON without starting the interactive/runtime path; this was tested. Treat it as a private packaged interface with a capability probe, not a new generalized SDK or a guessed stable contract.
- Readback should validate exact ID, ordered text digest and native projection; additionally preflight the known user-prefix filter. Separate native readback, native UI visibility, next-request capture and real response stages.

## Selected evidence and reproduction

Directory: `../evidence/gemini-native/`.

- `results.json`: import/list/restart statuses, exact original transcript projection, actual next-turn URL/model context, direct native readback results.
- `finish-result.json`: installed processor missing-finish counterexample and prefix filtering cases.
- `source-manifest.json`, `version-0.46.txt`: precise executable/package/source identity.
- `setup_probe.py`, `probe.py`, `read-native.mjs`, `probe-finish.mjs`: bounded synthetic reproductions. Run setup in a fresh scratch directory first, then probe. Node helpers must use the generated isolated `env.json`; never point them at a real Gemini home. Existing Homebrew 0.46.0 is required; scripts install nothing.

Selected evidence omits target system prompts/tool schemas and all credential/config payloads. Scratch includes only synthetic session data. The first listing experiment wrote the CLI's own synthetic error diagnostic into the OS temp directory before TMPDIR was redirected into scratch; no real content or secret was involved. Subsequent runs route temporary diagnostics into scratch.

Not proven: real model reply, interactive visual replay, Windows execution, npm-global 0.59.0 compatibility, exact continuation of prefix-filtered user content, or final-only extraction from ordinary raw Gemini history. No production capability should be upgraded beyond these boundaries.
