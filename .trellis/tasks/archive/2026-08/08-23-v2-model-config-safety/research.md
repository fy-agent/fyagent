# V2 Models 配置安全与连通性修复 — Research Record

## 1. Repository findings

### Provider Quick Setup write path

- `src-tauri/src/commands/provider.rs` constructs minimal Claude, Codex, and
  Grok Build Provider snapshots for V2 Quick Setup.
- `ProviderService::apply_quick_setup` persists those snapshots and currently
  routes them through the generic live-file writer.
- The resulting live writes replace the whole Claude `settings.json`, Codex
  `config.toml`, or Grok Build `config.toml` projection. This is the direct
  cause of unrelated user configuration disappearing after V2 Quick Setup.
- WorkBuddy and OpenCode already use read-modify-write over the current file and
  already own one deterministic rolling backup. They are reference
  implementations for ordering and failure behavior, not candidates for a
  broad rewrite.

### Renderer findings

- Provider Quick Setup, WorkBuddy, and OpenCode derive the `待保存` badge from
  non-empty fields/drafts rather than from a committed baseline.
- `ModelConnectivityTest` owns its last result internally and has no parent
  invalidation signal, so a pre-save failure can remain visible after a
  successful save.
- Claude, Codex, Grok Build, WorkBuddy, and OpenCode all reuse the shared V2
  `SecretInput`; the reported reveal-button drift must therefore be fixed and
  tested at the shared owner.

## 2. Temporary protocol probe

- A planning-time A/B comparison isolated one concrete Codex probe defect: an
  extra output-limit field in the current Responses request causes a
  compatibility failure on an otherwise working endpoint.
- The input representation was not demonstrated to be the triggering variable.
- Temporary probe files were removed; no product source or user configuration
  file was modified by the check.
- Endpoint, model and credential details are intentionally omitted from Trellis.
  Implementation proof will use exact local wire-level regression tests rather
  than another live external API call.

## 3. Protocol ownership findings

- Codex Quick Setup writes `wire_api = "responses"`; its probe must use the
  Responses endpoint.
- Grok Build Quick Setup writes `api_backend = "responses"`, but current
  `ModelProbeApp::GrokBuild` uses Chat Completions. This is an independent,
  deterministic protocol mismatch.
- Claude remains Anthropic Messages.
- WorkBuddy and OpenCode remain OpenAI-compatible Chat for their current
  managed configuration shapes.

## 4. Planning-artifact audit

File modification times showed that only this new task directory changed after
the task was created. No tracked product source file had been accidentally
edited during planning or probing.

The late-modified planning artifacts contained an over-broad statement that
extended the strict A/B result beyond what was actually isolated. The task now
records only the protocol-level conclusion needed for implementation.
