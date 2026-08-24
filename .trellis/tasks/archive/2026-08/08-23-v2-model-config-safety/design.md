# V2 Models 配置安全与连通性修复 — Technical Design

## 1. Confirmed root causes and evidence

### 1.1 Provider Quick Setup currently replaces whole live files

- `ProviderQuickSetupRequest::into_provider` builds a minimal Provider snapshot for Claude, Codex, and Grok Build.
- `ProviderService::apply_quick_setup` then routes that snapshot through the generic Provider live writer.
- Codex ultimately reaches `write_codex_live_atomic` / `write_codex_live_config_atomic`, which atomically replaces the complete `config.toml` text.
- Claude writes a complete `settings.json` value derived from the minimal Provider settings.
- Grok Build writes the complete `~/.grok/config.toml` snapshot.

This explains the reported Codex reduction from a large user-owned file to the small Quick Setup template. The bug is not a renderer-only issue.

### 1.2 Existing project primitives already support safer mutation

- Codex and Provider code already depend on `toml_edit::DocumentMut` and have syntax-preserving merge/update helpers.
- Grok Build already has `toml_edit` field mutation helpers.
- Provider live code already has structural JSON/TOML merge helpers.
- WorkBuddy already implements the desired backup ordering: validate and re-read the preimage, write one fixed backup, fail closed on backup failure, then replace the primary file.
- OpenCode Models already performs read-modify-write over the existing JSON object and writes one fixed `opencode.json.backup` before the primary write.

No new parser, form framework, or third-party UI dependency is required.

### 1.3 Save-state and stale probe bugs are renderer state bugs

- `ProviderPanel` marks `pending` from `Boolean(baseUrl || apiKey || modelId)`, so a successful save that keeps the values visible remains permanently “待保存”.
- WorkBuddy and OpenCode use the same non-empty-draft style and can exhibit the same stale pending semantics after a successful write.
- `ModelConnectivityTest` owns its previous result internally; parent save success cannot invalidate that result, so a pre-save failure can remain visible after the configuration changes.

### 1.4 `SecretInput` is already the shared owner

- Codex/Claude/Grok Build, WorkBuddy, and OpenCode reuse `src/v2/shared/ui/SecretInput.tsx`.
- The reveal button is absolutely positioned by the shared V2 controls stylesheet.
- The exact geometry defect must be reproduced in Chromium/WebKit-style layout before choosing the CSS correction; the fix belongs in this shared component/style, not in a Codex-only selector.

### 1.5 The model probe does not match the configured native protocols

- Codex is configured with `wire_api = "responses"`, but the current FyAgent probe sends a custom Responses body containing `max_output_tokens = 16` and a simplified string `content` shape.
- Current upstream Codex `ResponsesApiRequest` does not contain `max_output_tokens`; its user input uses typed `input_text` content and includes the normal Responses request control fields.
- Codex static provider headers are part of the provider contract. The existing probe cannot reproduce `x-openai-actor-authorization` when the V2 image extension is enabled.
- Grok Build Quick Setup explicitly writes `api_backend = "responses"`, while `model_probe.rs` currently maps `GrokBuild` to Chat Completions. This is a confirmed cross-layer mismatch.
- OpenCode Models uses `@ai-sdk/openai-compatible`, so Chat Completions remains the correct probe family there. WorkBuddy is also currently an OpenAI-compatible chat-style target. Claude remains Anthropic Messages.

A temporary planning-time protocol comparison isolated one concrete request-shape defect: the current Codex probe adds an output-limit field that is not required by the native Responses request and causes compatibility failures on at least one otherwise working endpoint. The input representation itself was not demonstrated to be the trigger. Implementation therefore removes that field and locks the exact request shape with local wire-level tests. No real endpoint, model alias, or credential is part of the task contract.

## 2. Architecture and ownership

### 2.1 Preserve Provider SSOT; specialize only the Quick Setup live projection

Do not rewrite the legacy Provider switching architecture globally.

Claude/Codex/Grok Build V2 Quick Setup keeps its stable reserved Provider row and existing DB/current transaction. The change is at the Quick Setup live projection boundary: the live file is derived by applying **owned Quick Setup fields to the current live preimage**, rather than treating the minimal Provider snapshot as the entire live file.

Generic imported Providers and existing switch/proxy flows keep their current snapshot semantics unless they consume the fixed Quick Setup reserved row, in which case they must reuse the same Quick Setup patch projection so a later switch back cannot reintroduce full-file clobbering.

### 2.2 Owned fields

#### Claude Quick Setup

Only these values are Quick Setup-owned in the live `settings.json`:

- `env.ANTHROPIC_BASE_URL`
- `env.ANTHROPIC_AUTH_TOKEN`
- `env.ANTHROPIC_MODEL`

All other top-level settings and other `env` members are preserved.

#### Codex Quick Setup

The live `config.toml` patch owns:

- top-level `model_provider = "custom"`
- top-level `model = <submitted model>`
- `[model_providers.custom]` fields created by Quick Setup:
  - `name`
  - `base_url`
  - `wire_api = "responses"`
  - `requires_openai_auth`
  - `supports_websockets` (present only when enabled; a previously Quick Setup-owned value is removed when disabled)
  - `experimental_bearer_token` only when image extension semantics require it
  - only the `x-openai-actor-authorization` entry inside `http_headers`; other headers survive

`disable_response_storage` is not an editable V2 Quick Setup field. Preserve an existing user value. For a brand-new config, retaining the current Quick Setup default is allowed, but an existing explicit value is never overwritten merely by saving a model endpoint.

All unrelated top-level keys, other model providers, comments, ordering, MCP, features, projects, hooks, desktop, memory, history, notification, sandbox/approval and user-defined tables survive.

Codex `auth.json` keeps the existing project security semantics: image-extension-off may update the auth key; image-extension-on uses the provider-scoped bearer path already specified by the project. This task does not change credential ownership merely to reduce the number of files touched.

#### Grok Build Quick Setup

Patch only:

- `[models].default`
- the selected `[model.<submitted id>]` fields owned by Quick Setup: `model`, `base_url`, `name`, `api_key`, `api_backend`, and the current Quick Setup `context_window` default.

Other model entries, extra fields in the selected model table, MCP, CLI/UI/sandbox/features and all other TOML remain untouched. Changing the selected model does not delete older model blocks unless ownership can be proven from the stored Quick Setup row; preservation is preferred to guessing that a block is disposable.

### 2.3 One rolling backup per physical file

For each physical user file that the operation will actually mutate:

1. validate/parse the current preimage;
2. derive the patched output;
3. immediately before the primary mutation, create or replace one deterministic FyAgent backup next to the file;
4. if any required backup fails, perform no primary mutation;
5. then use the existing atomic write/rollback path for the primary files.

Provider Quick Setup uses a deterministic suffix owned by FyAgent (for example `<filename>.fyagent.backup`) rather than timestamp generations. Repeated saves overwrite the same backup with the immediately previous valid preimage.

Existing WorkBuddy `models.json.backup` and OpenCode `opencode.json.backup` remain their single rolling backup owners; do not create a second backup system for them.

Backups containing credentials must follow the same user-scope and restrictive-permission boundary as the source file. Windows must continue to use the project's existing user-scope storage authority rather than an elevated-process shortcut.

### 2.4 Pre-write path disclosure

Every V2 Models panel that can mutate a local model configuration must show a shared disclosure before the save control can mutate data:

```text
将修改：<authoritative target path>
保存前备份：<authoritative single backup path>
仅保留这一份备份；再次保存会更新它。
```

When Codex will also update `auth.json`, show that file and its backup as a second entry. When a target file does not yet exist, the copy explains that there is no preimage to back up, while still showing the deterministic backup location that will be used after the file exists.

Paths come from native path owners; React never constructs `~/.codex`, `.workbuddy`, or platform-specific paths. If authoritative path metadata is unavailable, saving is disabled rather than proceeding without the required disclosure.

Reuse one Models-route disclosure component in `modelsShared.tsx`; do not create one per target.

### 2.5 Dirty-state transaction model

Replace “non-empty means dirty” with a small Models-shared commit-revision state:

- every user-owned draft mutation increments a local draft revision;
- a save captures the submitted revision;
- only successful persistence marks that submitted revision committed;
- edits made while a save is in flight remain dirty because their revision is newer;
- clearing an API Key from memory after submission is programmatic cleanup, not a new edit;
- a failed/rolled-back save never advances the committed revision.

ProviderPanel, WorkBuddy and OpenCode reuse this mechanism. Header `pending` is exactly `draftRevision != committedRevision`.

### 2.6 Connectivity-result invalidation

`ModelConnectivityTest` remains the one shared picker/result component. Add a non-secret reset version from the owning panel. Clear its old result when:

- connection-defining draft state changes;
- a successful save commits a new baseline;
- the target context is otherwise refreshed authoritatively.

No API key is placed in a React key, DOM attribute, query key, URL, log, or error text.

### 2.7 Protocol-correct real model probes

Keep one bounded native `stream_check_model` command, but make app semantics match the configuration produced by the corresponding Models panel.

| Target     | Probe protocol          | Important details                                                                                                                    |
| ---------- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Claude     | Anthropic Messages      | Existing Anthropic headers/auth path                                                                                                 |
| Codex      | OpenAI Responses        | Minimal native-compatible Responses request; omit the confirmed incompatible output-limit field; bounded provider header intent only |
| Grok Build | OpenAI Responses        | Must match Quick Setup `api_backend = "responses"`                                                                                   |
| WorkBuddy  | OpenAI Chat Completions | Existing OpenAI-compatible chat path                                                                                                 |
| OpenCode   | OpenAI Chat Completions | Matches the managed `@ai-sdk/openai-compatible` provider                                                                             |

The renderer may pass only the bounded Codex feature intent needed to reproduce its own saved request. It may not submit arbitrary headers, protocol strings, or endpoint paths.

Success requires an HTTP success response and a non-error first stream event/chunk. Error bodies remain bounded and credential-redacted.

## 3. Data flow and contracts

### Save flow

```text
V2 panel draft
  -> authoritative path/backup disclosure already loaded
  -> minimum save request (secret only in memory/IPC)
  -> per-app native config lock
  -> validate request and current live preimage
  -> derive targeted patch
  -> write/replace the one required backup per existing physical target
  -> atomically persist live + DB/current state under existing compensation rules
  -> reread sanitized authority
  -> renderer commits submitted draft revision
  -> clear API key memory + stale connectivity result
```

### Probe flow

```text
V2 model picker
  -> bounded { app, baseUrl, apiKey, modelId, codexFeatureIntent? }
  -> native URL/model validation
  -> app-owned request projection
  -> authenticated streaming request
  -> bounded result without credentials
```

## 4. Compatibility and migration

- No database schema migration.
- Stable Quick Setup Provider IDs remain unchanged.
- Existing generic Provider records remain readable.
- Existing WorkBuddy/OpenCode backup filenames remain unchanged.
- No automatic rewrite is performed on application startup. The safer mutation path applies only when the user explicitly saves/activates a model configuration.
- Existing malformed config still fails closed; FyAgent never “repairs” a malformed user file by replacing it with a minimal template.

## 5. Rollback and failure behavior

- Invalid current config -> reject before backup and write; show a controlled configuration-read/parse failure.
- Backup failure -> reject with target unchanged.
- Primary write failure -> use existing atomic/compensation path; persistent backup remains available.
- DB/current/live compensation failure -> preserve existing structured partial-state outcome; never report success.
- Native summary/path reread failure after a successful write -> report unconfirmed authority without reusing the old connectivity result.
- Connectivity failure never blocks a save by itself; it is diagnostic. Saving does not reinterpret a failed probe as successful.

## 6. Reuse decisions

- Reuse `SecretInput`; fix its shared geometry contract rather than creating a password component.
- Reuse `ModelConnectivityTest`; add reset/protocol intent rather than creating Codex/Grok-specific probe widgets.
- Reuse `modelsShared.tsx` for the write disclosure and draft-commit state used by multiple Models panels.
- Reuse `toml_edit` and existing Provider JSON/TOML structural helpers for targeted configuration edits.
- Reuse WorkBuddy/OpenCode fixed-backup patterns; do not add a backup package.
- Do not import leftover V1 UI into V2. Leftover business/command behavior may be used as reference through existing V2 FeaturePorts.

## 7. Residual risks

- An external editor can still race an atomic rename at the filesystem boundary. The implementation should re-read/compare the validated preimage immediately before backup/write where the existing target storage permits it, but no advisory lock can force an unrelated editor to cooperate.
- Protocol acceptance is based on exact local request/response fixtures and
  wire-level tests. No external live endpoint is required for implementation
  completion unless the user separately asks for another manual probe.
- Windows path/ACL behavior cannot be claimed from macOS-only execution; existing Windows-native CI/security contracts remain required evidence.
