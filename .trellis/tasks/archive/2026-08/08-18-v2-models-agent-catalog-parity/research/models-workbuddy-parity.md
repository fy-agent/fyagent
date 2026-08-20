# Models page: WorkBuddy parity evidence (2026-08-18)

## WorkBuddy baseline (keep)

WorkBuddy already has the complete loop in V2:

- Read existing third-party model IDs
- Connection fields (base URL, API key, allow no key)
- Fetch remote `/v1/models`
- Draft list with add / remove / search / grouped chips
- Immediate delete of an existing ID after unrecoverable-delete confirmation
- Save with revision / overwrite token
- Secrets never enter URL, storage, or query cache

Leftover `src/components/workbuddy/**` is the older visual, not a second persistence contract. V2 must keep using the WorkBuddy native commands.

## TRAE Work CN — production write path

Official docs (`https://docs.trae.cn/work_models`) confirm TraeWork desktop custom models: API format, base vs complete URL, model ID, API key, vendor-side probe on submit. They do **not** publish a file schema.

Local TRAE SOLO CN evidence:

- Custom models are stored in `~/Library/Application Support/TRAE SOLO CN/User/globalStorage/state.vscdb`
- Key pattern: `<machine>:AI.agent.model.model_list_map`
- Work-mode lists: `solo_work_lite`, `solo_work_remote`
- Each row is a large object. Secret carriers include `ak` and `sk`. Preset rows have `is_preset: true`.
- This machine currently has **zero** custom rows, so a live custom template cannot be copied from production data.

Formal scheme for this task (not probe-only, not “测试”):

1. **Read**: open the vscdb read-only, project only non-secret custom rows (`is_preset == false`) from `solo_work_lite` (canonical). Fields to V2: `modelId` (`custom_model_id` or `name`), `displayName`, `baseUrl` if non-secret. Never serialize `ak`/`sk`.
2. **Fetch**: reuse the existing TRAE URL admission + OpenAI-compatible `/models` fetch (or `fetch_models_for_config` equivalent behind a TRAE-owned command). Probe remains the connection test, not the save.
3. **Save/delete**: backup the JSON blob, clone one existing Work-mode preset object as a structural template, override identity/connection fields (`is_preset=false`, `is_custom_base_url=true`, `name`, `display_name`, `custom_model_id`, `base_url`, `ak`, `is_default=false`, `selectable=true`, `status=true`), append or remove matching custom rows in **both** `solo_work_lite` and `solo_work_remote`. Never mutate `is_preset=true` rows. HMAC revision + one-time overwrite token, WorkBuddy-style.
4. Tests use a fixture sqlite, never the interactive user’s TRAE profile.
5. Windows path must go through the same Explorer-user / reparse rules as other user-profile files.

If a write cannot be proven against a fixture that TRAE will parse, fail closed and do not claim save. Do not keep “请回 TRAE 保存” as the happy path.

## OpenCode — reuse leftover, expose in V2

Leftover `OpenCodeFormFields` already supports fetch / add / delete models and writes `opencode.json` via `opencode_config.rs`. V2 currently only shows vendor-UI guidance and the catalog marks `models.write` as `assisted`.

Formal scheme:

- New V2 OpenCode models port (do not import leftover React):
  - `get_opencode_model_snapshot` → sanitized provider ids + model ids (no keys)
  - `fetch_opencode_provider_models({ baseUrl, apiKey, allowNoApiKey })`
  - `save_opencode_models({ ... revisioned request ... })` using existing OpenCode provider live sync
- UI copies WorkBuddy: existing ids, connection, fetch, draft add/delete, save.
- `get_opencode_models` (CLI runtime list) may seed the draft; it is not a substitute for writing `opencode.json`.

## Claude Code — fetch + model chips on the reserved Provider

Leftover Claude forms already call `fetch_models_for_config`. V2 `ProviderPanel` is name / URL / key / one `modelId`.

Formal scheme:

- Keep the reserved quick-setup Provider (do not revive generic add/update/switch in React).
- Add fetch → grouped chips with vendor icons; user can add manual ids and remove draft ids.
- Save still goes through `applyQuickSetupWithResult`. Current model is the selected chip (exactly one required). Extra fetched ids stay draft-only unless the reserved Claude shape already stores a list; do not invent a second Claude model table.
- If the sanitized summary needs a non-secret `modelId`, add it to `ProviderSummary` without widening secret projection.

## Codex

Keep managed installer + existing quick setup. Add the same tiny vendor icon beside any shown model id. Do not rebuild Codex as a WorkBuddy clone in this task.

## Model vendor icons

Leftover `src/icons/extracted/` already contains vendor SVGs (`openai`, `claude`, `anthropic`, `deepseek`, `qwen`, `gemini`, `grok`, `kimi`, `minimax`, `mistral`, `meta`, `doubao`, `chatglm`, `ollama`, `openrouter`, …). Leftover UI groups fetch results by `ownedBy` but does **not** render those icons next to model ids.

V2 must copy the needed SVGs into `src/v2/shared/assets/models/` (V2-owned paths; no import from `src/components` or `src/icons`). Resolve by `ownedBy` then model-id prefix. Unknown vendors get a neutral glyph, never a remote URL.
