# Native continuation verification — 2026-09-22

Author: native-continuation independent executor. Production files were read only; backend writer remained the sole owner. All fixtures were synthetic and stored under the task scratch directory. No real session or credential files were opened, no provider inference was performed, and no dependencies were installed or upgraded. Native continuation model traffic was limited to a loopback HTTP capture server, which deliberately returned HTTP 400 before inference. Hermes `--version` performed its ordinary update-availability check; no update was run.

## Results and implementation decisions

| Path | Actual result | Consequence |
| --- | --- | --- |
| OpenCode current placeholder model projection | Official import/export succeed, preserving five messages exactly. Bare `run --session` fails before a model request with `ProviderModelNotFoundError: fyagent-migration/fyagent-migration`. | Current writer cannot claim ordinary continuation works. |
| OpenCode explicit target local `--model probe/local` | One next-turn request reaches the loopback server. All five historical roles/texts are exact, including consecutive users, consecutive finals, CRLF, Chinese, Windows path and trailing unanswered user. | Native history is usable; the problem is target model selection. |
| OpenCode **target local `info.model`** | Set session header `model: { providerID: "probe", id: "local" }`; keep historical model placeholders. Bare `run --session` now reaches the loopback server with the same exact history and `model: "local"`. | Smallest verified fix: resolve target local model before import and set session header model. No source model/account is required. |
| Hermes current foreign-session import | Official `sessions import --from codex` + official `sessions export - --session-id ID` works for ordinary alternating text. The five-message edge fixture becomes three messages; surrounding whitespace/CRLF are stripped and same-role messages merged. | Foreign import is not a lossless writer for the product contract. Current unconditional write should not be presented as a complete migration. |
| Hermes official Python `SessionDB.create_session` + `append_message` | Writes all five raw bodies exactly. A separate official CLI export returns all five bodies exactly; `model` and `model_config` are null. | A version-pinned native SDK writer can preserve stored data without custom SQL or copying source settings. |
| Hermes official native resume projection | `get_resume_conversations` display history keeps five message boundaries but strips edges; model history strips and merges to three messages. | SDK writer does not by itself prove exact native resume/render or exact next-turn request. Keep those stages unverified or explicitly blocked for affected content. |

## Versions and direct evidence

- OpenCode **1.18.30**, installed `/Users/serendipity/.opencode/bin/opencode`.
- Hermes **v0.20.5 (2026.8.19)**, upstream `f293e720`, local `987064caa4f8845f605ac7346fed5b72fddfb21c` (+1 carried commit), installed official package/venv under `/Users/serendipity/.hermes/hermes-agent`.
- Selected machine-readable result: `../evidence/native-continuation/results.json`. Target runtime system prompts, tool schemas and full config are omitted from selected evidence.
- Reproduction scripts: `probe_opencode.py`, `probe_hermes_sdk.py` in that evidence directory. Scripts expect the isolated environment JSON prepared in scratch; rerun instructions below. Full scratch is `/Users/serendipity/Documents/Codex/2026-09-22/new-chat/work/native-continuation`.
- No genuine next-turn model response, interactive TUI screen inspection, Windows execution or other CLI version was tested. HTTP 400 in successful capture variants is deliberate and **not** a continuation failure.

## OpenCode smallest repair

`native/opencode.rs:265` writes historical user.model = `fyagent-migration/fyagent-migration`. The imported session header currently has no `model` field. Version-pinned official source `packages/opencode/src/session/prompt.ts:615–633` resolves model in this order: session.model, latest historical user.model, provider default. Therefore even target `config.model = probe/local` does not override the placeholder in a bare continuation. The log capture `opencode-default-error-lines.txt` confirms the exact exception.

The verified fix is:

1. Before any native write, run target CLI `opencode debug config --pure` **with the selected target workspace as cwd and the same target store/config environment as import**. It is a non-inference official command. Parse JSON internally and retain only resolved `model`; do not log/persist the full output, which may contain credentials or provider options.
2. In the fixture header set `info.model = { providerID: prefix_before_first_slash, id: remainder_after_first_slash }`. `id`, not `modelID`, is the session-header field; historical message model still uses `modelID`. Never use source model values.
3. If a usable target default is absent, fail before writing with an actionable local-model-selection reason or accept a locally selected model parameter. Do not invent a model. `debug config` does not prove the selected model is actually installed/authorized; normal target model validation remains required.
4. Historical placeholders can remain because the header now controls the default. Setting them to the target local model is also logically possible, but was not needed for the passing experiment. Target-local `info.model` is the tested minimum.

The test used target config `enabled_providers: ["probe"]`, bundled `@ai-sdk/openai-compatible`, synthetic API key and `http://127.0.0.1:<port>/v1`. All non-loopback HTTP proxy routes pointed at a closed local port. `debug config` returned `model: "probe/local"`; selected evidence contains only that field. The request body contained the five historical messages as five distinct entries, byte-for-byte exact.

Official version source:

- https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/session/prompt.ts
- https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/cli/cmd/import.ts
- https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/cli/cmd/run.ts

## Hermes smallest repair and remaining boundary

The current `native/hermes.rs:175–190` readback blocker is factually stale. Installed CLI help and real execution prove **`hermes sessions export - --session-id FULL_ID --format jsonl`** provides a non-interactive official readback. Parse the one-session JSON object, require exact `id` match (the CLI accepts prefixes), project its `messages` ordered role/content, and compare the content digest. Do not mistake exit 0 with a human-readable error for successful JSON readback.

The installed importer in `hermes_cli/foreign_sessions.py:99–155,235–273` strips plain text, skips recognized wrapper-like user messages and merges consecutive same-role turns. This means a pre-filtered FyAgent transcript is still changed. The edge probe gave:

```text
input  U:" First\r\n\n", U:"Second", A:" Third  ", A:"Fourth", U:"Unanswered"
output U:"First\n\nSecond", A:"Third\n\nFourth", U:"Unanswered"
```

There is an official installed SDK path that avoids the foreign parser:

```python
from hermes_state import SessionDB
db = SessionDB()  # HERMES_HOME resolves the selected target store
db.create_session(preallocated_native_id, source="cli", cwd=target_workspace)
for role, text in transcript:
    db.append_message(preallocated_native_id, role, text)
db.close()
```

It was exercised using the installed Hermes venv Python with only the installed source directory in `PYTHONPATH`; the CLI wrapper points into that same installation. A production adapter must discover/validate the selected installation instead of hardcoding this Mac path. Do not install another Python package, pass secrets, or execute arbitrary session content as code. Supply data through a controlled JSON channel and use fixed code. Preallocate the new ID and persist it to FyAgent receipt before calling the SDK. `create_session` alone is not a no-overwrite guarantee: reject an existing exact ID before append and retain the normal pending/reconciliation discipline for partial writes.

This SDK API is part of the installed implementation, **not a promised stable external CLI contract**. Gate it by the verified installed version and use official CLI export as the independent readback. This removes the need for a custom schema writer and preserves stored messages exactly.

However, the official `hermes_state.py:11092–11109` resume decoder executes `sanitize_context(content).strip()` for user and assistant strings. Its `get_resume_conversations` also invokes the official alternation-repair routine for model history. The SDK probe therefore observes raw export exact, display projection not exact, model projection not exact. `api_content` is an existing byte-fidelity field, but changing it does not establish native display fidelity or same-role boundaries; it was not used or claimed as a complete fix. If the product requires exact rendering and next-request boundaries for all messages, this remains an upstream native-resume limitation. Do not quietly trim the package, insert dummy answers, or claim `nextTurnRequestVerified` for the Hermes edge case.

There is also a current crash-reconciliation defect: Hermes import stores the synthetic nonce in **`sessions.origin_json.imported_from.foreign_session_id`**, verified by official export. `find_sessions_with_marker` searches id/title/source/source_id/metadata/name, excluding origin_json, and `NonceCarrier::SessionSlug` misdescribes the actual location. A lost stdout ID will therefore not be recovered by the existing search. Prefer the preallocated SDK ID route; if foreign import remains, parse exact JSON nonce from origin_json rather than broad LIKE across text columns.

Official public export documentation corroborates the locally executed command: https://hermes-agent.nousresearch.com/docs/user-guide/sessions . Local installed source is authoritative for the exact v0.20.5 behavior used above.

## Reproduction

Use the scratch directory and its `env.json`, whose HOME/XDG/HERMES_HOME values point only at synthetic stores. `probe_opencode.py` runs three isolated variants and responds to loopback calls with HTTP 400. `probe_hermes_sdk.py` runs with the existing Hermes venv Python and `PYTHONPATH=/Users/serendipity/.hermes/hermes-agent`. These scripts do not alter product code. Copying scripts without also setting an isolated environment is not a valid reproduction and must not be run against real user stores.

Result status: sufficient to unblock a precise OpenCode implementation fix and Hermes official readback/SDK choice; **not** full Hermes continuation acceptance. Prior claims that an arbitrary historical model is harmless or Hermes lacks official non-interactive export are superseded by these observations.
