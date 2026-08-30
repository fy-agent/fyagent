# Research: current SuperGrok → Codex path

- **Query**: How xAI / SuperGrok device-code OAuth binds to a Codex provider; whether Change Plan / #63 can write it; what V2 shows with/without an xAI account; single-target demo path using existing owners; HIL vs real SuperGrok account.
- **Scope**: mixed (internal code + parent GitHub decisions #106 / #42 / #41 / #63)
- **Date**: 2026-08-31

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src-tauri/src/commands/auth.rs` | Shared managed-auth IPC: `auth_start_login` / `auth_poll_for_account` / list / status / logout for `xai_oauth` |
| `src-tauri/src/proxy/providers/xai_oauth_auth.rs` | Device-code OAuth manager; tokens in `xai_oauth_auth.json` |
| `src-tauri/src/commands/xai_oauth.rs` | Quota + models commands; not login |
| `src/lib/api/auth.ts` | Renderer IPC for `ManagedAuthProvider` including `xai_oauth` |
| `src/components/providers/forms/hooks/useXaiOauth.ts` | Thin wrapper: `useManagedAuth("xai_oauth")` |
| `src/components/providers/forms/XaiOAuthSection.tsx` | Device-code UI (Auth Center + Codex form) |
| `src/components/settings/AuthCenterPanel.tsx` | V1 Auth Center owner for xAI / Grok |
| `src/config/codexProviderPresets.ts` | Codex presets: API-key `xAI (Grok)` vs managed `xAI (Grok) OAuth` |
| `src/components/providers/forms/ProviderForm.tsx` | Binds `meta.providerType` + `authBinding` on save |
| `src-tauri/src/commands/provider.rs` | V1 `add_provider`; V2 `ProviderQuickSetupRequest` (API-key only) |
| `src-tauri/src/commands/change_plan.rs` | `create_codex_provider_upsert_plan` / `apply_change_plan` |
| `src-tauri/src/services/change_plan/service.rs` | Credential gate + reserved upsert id |
| `.trellis/spec/backend/change-plan-executor.md` | Closed adapters: switch / upsert / WorkBuddy only |
| `src/v2/pages/agents/AgentAuthStatusPanel.tsx` | Codex always delegated to Auth Center |
| `src-tauri/src/agent_install/auth_actions.rs` | Codex observation is static `fyagent_managed` |
| `src/v2/pages/models/Page.tsx` | Codex Quick Setup + Change Plan workspaces |
| `src/v2/pages/models/quickSetup.ts` | API-key form contract |

### 1. How xAI / SuperGrok device-code OAuth binds to a Codex provider

Login and token storage are **not** Codex-owned. They are the shared managed-auth lane `xai_oauth`.

**Commands (login / account):**

- `auth_start_login` with `auth_provider = "xai_oauth"` → `XaiOAuthManager::start_device_flow` (`src-tauri/src/commands/auth.rs:109-144`).
- `auth_poll_for_account` → `poll_for_token` (`auth.rs:147-199`).
- Also: `auth_get_status`, `auth_list_accounts`, `auth_set_default_account`, `auth_remove_account`, `auth_logout`, `auth_cancel_login`.
- xAI-specific extras (not login): `get_xai_oauth_quota`, `get_xai_oauth_models` (`src-tauri/src/commands/xai_oauth.rs:68-91`).

**Protocol owner:** `XaiOAuthManager` (`xai_oauth_auth.rs:18-22`, `220-250`). Device Authorization Grant against `https://auth.x.ai`. Refresh tokens persist in app-config `xai_oauth_auth.json` (`xai_oauth_auth.rs:211`). Provider config keeps a placeholder key only (`codex.rs:732-739`).

**Auth Center UI owner:** V1 Settings tab `auth` → `AuthCenterPanel` (`SettingsPage.tsx:305-312`, `AuthCenterPanel.tsx:75-90`). Section title is `xAI (Grok OAuth)`; it mounts `XaiOAuthSection`. Hook is `useXaiOauth` → `useManagedAuth("xai_oauth")` (`useXaiOauth.ts:4-6`, `useManagedAuth.ts:13-41`). Frontend IPC: `src/lib/api/auth.ts:3-6,38-46`.

**Codex presets (two, do not confuse):**

| Preset name | `providerType` | Auth | Owner |
|---|---|---|---|
| `xAI (Grok)` | absent | empty `OPENAI_API_KEY` — API-key path | `codexProviderPresets.ts:1390-1413` |
| `xAI (Grok) OAuth` | `xai_oauth` | empty key + `requiresOAuth: true` | `codexProviderPresets.ts:1414-1436` |

Preset contract test documents this split (`tests/config/xaiOauthProviderPresets.test.ts:58-111`). Claude Code / Claude Desktop use a single managed preset named `xAI (Grok)` with `providerType: "xai_oauth"` (`claudeProviderPresets.ts:1241`, `claudeDesktopProviderPresets.ts:751`). Parent PRD “already usable on Claude / Codex presets” refers to the **OAuth** Codex preset plus Claude presets — not the API-key Codex row.

**Bind on save (V1 Codex form):**

1. User picks `xAI (Grok) OAuth`. Form hides API Key and mounts `XaiOAuthSection` (`CodexFormFields.tsx:493-498`).
2. Save refuses if no usable xAI account (`ProviderForm.tsx:1176-1201`, `1234-1243`).
3. Payload writes (`ProviderForm.tsx:1552-1587`):
   - `meta.providerType = "xai_oauth"`
   - `meta.authBinding = { source: "managed_account", authProvider: "xai_oauth", accountId }`
4. Persist via `add_provider` / `add_provider_with_result` / update (`provider.rs:417-437`). Not Change Plan.
5. Runtime: `Provider::is_xai_oauth()` (`provider.rs:96-98`). `CodexAdapter` pins `XAI_API_BASE_URL` and `xai_oauth_placeholder` (`codex.rs:677-739`). Forwarder injects the live token from `XaiOAuthManager` (`forwarder.rs:1745`, `3284`).

Credentials are not copied into Codex `auth.json` or the Provider row. That already satisfies R3 if this bind path is reused.

### 2. Change Plan / apply path (#63) — can SuperGrok reuse it?

**Yes as the executor. No as the current request/admission shape.**

Registered operations (`change-plan-executor.md:10-31`, `adapter.rs:53-56`):

- `codex_provider_switch` ← `create_codex_provider_switch_plan(targetProviderId)`
- `codex_provider_upsert_and_switch` ← `create_codex_provider_upsert_plan(request)`
- `workbuddy_models_save`

Apply is one command: `apply_change_plan(planId, planDigest)` (`change_plan.rs:63-96`). Codex upsert writer is `ProviderService::apply_quick_setup_with_lock_held` (`change_plan.rs:91-95`). No second executor is required.

**Three hard gates block SuperGrok today:**

1. **Quick Setup DTO is API-key only.** `ProviderQuickSetupRequest` = `{ name, baseUrl, apiKey, modelId, codexFeatures? }` (`v2/shared/features/models.ts:3-12`, `provider.rs:201-211`). `into_provider` for Codex writes `OPENAI_API_KEY` + reserved id `fyagent-v2-quick-setup-codex` and never sets `providerType` / `authBinding` (`provider.rs:251-307`). Empty `apiKey` is rejected (`provider.rs:223-227`).
2. **Credential capability rejects managed OAuth.** `prove_codex_target_credential_capability` returns `SecretDependencyUnavailable` if `auth_binding.source == ManagedAccount` **or** any `provider_type` is set **or** `uses_managed_account_auth()` (`service.rs:1593-1646`). `xai_oauth` hits all three. Same function is used by switch (`service.rs:265-267`) and upsert (`service.rs:351-353`).
3. **Upsert id is reserved.** `plan_codex_upsert` requires `provider.id == fyagent-v2-quick-setup-codex` (`service.rs:332-334`, `provider/mod.rs:199`). A V1-created UUID “xAI (Grok) OAuth” provider cannot be created through upsert; it could only be **switched**, and switch is blocked by gate 2.

#41 is the visible apply/readback/recover job model. #63 is the Codex Provider vertical already landed on that executor (preview → `{ planId, planDigest }` confirm → `getChangeJob`). SuperGrok can stay on this adapter if implement adds a **narrow admission exception** for `xai_oauth` managed accounts (token stays in `xai_oauth_auth.json`; plan stays credential-free) plus a **create input that is not the API-key Quick Setup DTO**. Do not add a fourth adapter.

### 3. What V2 shows for Codex when an xAI OAuth account exists vs not

**Identical. V2 does not observe xAI accounts.**

| Surface | With xAI account | Without xAI account |
|---|---|---|
| Agent Codex auth | `kind: fyagent_managed`, copy「由 FyAgent 认证中心管理」, no 登录 button | Same |
| Agent Codex models | `get_provider_summary` names only (`id` + `name`) | Same |
| Models Codex form | API-key Quick Setup (name / URL / key / model) | Same |
| Models Change Plan | Switch other named Providers; upsert reserved slot from API key | Same |

Evidence:

- Codex observation is a constant: `observe_agent_auth(Codex) => fyagent_managed_observation()` (`auth_actions.rs:63-65`, `519-529`). It does not call `XaiOAuthManager`.
- Parser only allows `fyagent_managed` for `agentId === "codex"` (`agent-auth.ts:350-376`).
- UI: `AgentAuthStatusPanel.tsx:48-66`, `130-131`, test `AgentAuthStatusPanel.test.tsx:192-218`.
- Provider public summary is `{ id, name }` only (`provider.rs:20-23,195-198`). No `providerType`, no auth state.
- Models Codex save always `validateQuickSetup` (requires API key) then `createCodexProviderUpsertPlan` (`Page.tsx:1079-1140`).
- `src/v2/**` has **zero** `xai_oauth` / SuperGrok / Auth Center panel imports. V2 architecture forbids importing V1 `AuthCenterPanel` / `XaiOAuthSection` (`tests/v2/app/architecture.test.ts:157-171`).
- V2 settings control is a no-op (`ToolCluster.tsx` `onClick={noop}`). Auth Center remains V1 Settings only.

If the user already created a V1 Codex provider named `xAI (Grok) OAuth`, V2 Models will list that **name**. Selecting it for Change Plan switch will fail with `secret_dependency_unavailable`. Existence of an xAI account **without** a Codex provider row is invisible on V2.

### 4. Single-target SuperGrok → Codex demo using existing owners only

#42 rule: one Codex plan; failure must not claim other Agents changed. Current Change Plan already emits one Codex-only plan.

**Working today (V1 owners, no new executor):**

1. **Login** — V1 Settings → Auth → `AuthCenterPanel` / `XaiOAuthSection` → `auth_start_login("xai_oauth")` → device code → `auth_poll_for_account`.
2. **Choose Codex** — V1 Codex app → Add Provider → preset `xAI (Grok) OAuth` (`codexProviderPresets.ts:1414-1436`).
3. **Bind** — `ProviderForm` writes `providerType` + `authBinding` (`ProviderForm.tsx:1552-1587`). No plaintext token in the row.
4. **Write + current** — `add_provider_with_result` then V1 `switch_provider` (not Change Plan).
5. **Runtime readback** — live `~/.codex` projection uses placeholder + local proxy; token from `XaiOAuthManager`. Quota footer can read SuperGrok via `get_xai_oauth_quota` (V1 card, not V2).

This path already satisfies “one source, one Codex target.” It does **not** satisfy PRD R4 (visible V2 path from logged-in SuperGrok to Codex readback).

**V2-visible demo that still reuses the same owners (recommended MVP shape):**

1. Keep login on Auth Center (`xai_oauth` commands + `XaiOAuthSection`). Do not clone OAuth into V2 Agent Auth (Codex is already `managed_by_auth_center`).
2. On V2 Codex Models, do **not** reuse the API-key Quick Setup form. Add a thin native create that builds the existing OAuth preset + `authBinding` (account id only) and calls **existing** `ChangePlanService::plan_codex_upsert` **or** `plan_codex_switch` after V1 add.
3. Preview / confirm / poll stay `CodexSavePlanWorkspace` + `ApplyWorkspace` + `apply_change_plan` + `getChangeJob` (`CodexSavePlanWorkspace.tsx:55-107`).
4. Single target = Codex only. Do not emit Claude plans.
5. Readback = existing job phases (`precheck → snapshot → managed_write → readback → finalize`) plus `useProviderSummary("codex")` showing the named / reserved provider as current.

Admission change required: treat `xai_oauth` + `ManagedAccount` as `NoNewCredentialMaterial` when the account exists in `XaiOAuthManager` (secret not in the plan). Without that, Change Plan cannot preview or apply.

**Owner map (do not invent new ones):**

| Step | Existing owner |
|---|---|
| Device-code login | `auth_*` + `XaiOAuthManager` + Auth Center / `XaiOAuthSection` |
| Preset shape | `codexProviderPresets` `xAI (Grok) OAuth` |
| Bind fields | `ProviderForm` `authBinding` / `providerType` |
| Plan + apply + readback | `changePlans` port + `CodexExecutionAdapter` |
| Codex live write | `ProviderService::apply_quick_setup_with_lock_held` or V1 `switch_provider` |
| Token at request time | `CodexAdapter` + forwarder `XaiOAuthManager` |
| V2 Agent copy | `fyagent_managed` → Auth Center (keep; do not add a second login) |

### 5. HIL / fixture evidence vs real SuperGrok account

**Already exists (no live SuperGrok):**

- `xai_oauth_auth.rs` unit tests: identity, store round-trip, reauth, endpoint origin, error sanitization (`xai_oauth_auth.rs:1010-1175`). No live device-code call.
- Preset contract: `tests/config/xaiOauthProviderPresets.test.ts`.
- Locale / footer: `tests/config/xaiOauthLocales.test.ts`, `tests/components/XaiOauthQuotaFooter.test.tsx`.
- Codex adapter invariants with `provider_type: xai_oauth`: `codex.rs:1547-1602`.
- Change Plan fixtures: API-key upsert/switch only (`tests/v2/fixtures/changePlans.ts:13-28`). Capability tests prove managed binding is **rejected** (`service.rs:2590-2609`).
- V2 Agent Auth fixture: Codex → Auth Center, independent of xAI accounts (`AgentAuthStatusPanel.test.tsx:192-218`, `tests/v2-browser/support/features.ts:396-402`).
- Parent open question (`08-31-grok-first-class-iteration/prd.md:50`): accept via contract/fixture/handoff vs real SuperGrok HIL — not decided.

**Requires a real SuperGrok account:**

- Device-code against `auth.x.ai` (user_code, verification_uri, consent).
- Refresh-token persist / `requires_reauth` after revoke.
- `get_xai_oauth_models` / `get_xai_oauth_quota` against live grok.com / api.x.ai.
- End-to-end: bind → Change Plan preview → apply → Codex live projection → one real Codex request through the local proxy (namespace flatten + sanitizer).
- V2 “logged-in SuperGrok → Codex 已回读” UI, once it exists.

Contract + fixture can prove admission, reserved-id, no-secret-in-plan, and single-target apply. They cannot prove the OAuth handshake or a working Grok session.

### Related Specs

- `.trellis/spec/backend/change-plan-executor.md` — closed adapters; no renderer-supplied write target; plans stay credential-free.
- `.trellis/spec/frontend/v2-agent-models.md` — Codex Models = Quick Setup + Change Plan; Agent Codex auth stays Auth Center.
- Parent research: `.trellis/tasks/08-31-grok-first-class-iteration/research/github-decision-106.md`, `github-decision-42.md`.

## Confirmed facts

- SuperGrok login is FyAgent-managed `xai_oauth` device-code, stored in `xai_oauth_auth.json`, owned by Auth Center + `auth_*` commands.
- Codex bind is V1-only: preset `xAI (Grok) OAuth` → `meta.providerType=xai_oauth` + `authBinding` → `add_provider` / `switch_provider`.
- Codex runtime already injects the managed token; no second proxy/executor needed.
- V2 Change Plan (#63) is the apply/readback owner for Codex Provider, but it currently admits **API-key Quick Setup only** and **rejects** any `xai_oauth` / managed binding.
- V2 Codex UI does not change when an xAI account appears. Auth is always “go to Auth Center.”
- #42 single-target is already how Change Plan works (one Codex plan). Claude is a separate later reuse, not this task.

## Reuse owners

- Login: `auth_start_login` / `auth_poll_for_account` / `XaiOAuthManager` / `XaiOAuthSection` / `AuthCenterPanel`.
- Preset + bind: `codexProviderPresets` `xAI (Grok) OAuth` + `ProviderForm` `authBinding`.
- Apply: `create_codex_provider_*_plan` + `apply_change_plan` + `CodexExecutionAdapter` + `CodexSavePlanWorkspace` / `ChangePlanWorkspace`.
- Live write: existing `ProviderService` Codex writers.
- V2 Agent: keep `fyagent_managed` → Auth Center; do not start a second OAuth UI.

## Recommended MVP changes

1. **Admission exception** in `prove_codex_target_credential_capability` for `provider_type == xai_oauth` + `ManagedAccount` when `XaiOAuthManager` has a usable account. Plan/job still carry no token.
2. **Create input for that preset**, not an API-key Quick Setup fork: account id + display name (or reserved-slot upsert). Still one adapter.
3. **V2 visible stitch only:** from Codex Models / Agent, a path that (a) states Auth Center login if no xAI account, (b) previews one Codex plan, (c) confirms with `{ planId, planDigest }`, (d) readback via `getChangeJob` + provider summary. Do not import V1 form components into `src/v2`.
4. Prefer **switch of a V1-created xAI Codex provider** or **reserved-slot upsert**, not a new provider-id scheme.
5. Acceptance can be contract/fixture first; mark live SuperGrok as residual HIL unless the parent decides otherwise.

## What must stay out of scope

- Second executor / new Change Plan operation / renderer-supplied write target.
- Same-plan multi-agent apply. Claude Code / Claude Desktop are in this child as **separate** writes, not one shared plan. Claude has no Change Plan adapter; reuse V1 bind.
- WorkBuddy save (`08-31-grok-supergrok-to-workbuddy`).
- Login trichotomy copy (`08-31-grok-login-trichotomy`).
- Grok Build install/upgrade; V2 SuperGrok quota dashboard.
- Writing or treating `~/.grok/auth.json` as login proof.
- Putting access/refresh tokens in Provider rows, Quick Setup payloads, or Change Plan ledger.
- Cloning Auth Center OAuth chrome into V2 Agent Auth.
- Closing #42 / #63 / #41 wholesale.

## 2026-08-31 scope addendum

Parent iteration now includes Claude Code, Claude Desktop, Codex, and WorkBuddy. This research file’s Codex facts still stand. Claude / Desktop stay on existing V1 `xai_oauth` bind. WorkBuddy is a different save path and is documented in the sibling task.

## Caveats / Not Found

- No V2 port or command exists to list `xai_oauth` accounts. Implement must add a small read or keep login on V1 Auth Center and only consume “already logged in” on the native side.
- `ProviderPublicSummary` cannot tell V2 that a listed name is SuperGrok vs API-key without a new sanitized field or a dedicated create path.
- Upsert overwriting `fyagent-v2-quick-setup-codex` would replace the user’s V2 API-key Quick Setup slot; switch of a separate V1 provider avoids that collision.
- No SuperGrok HIL transcript was found in this task or the parent research folder.
- `task.py current` was unset; output was written to the user-specified task dir `.trellis/tasks/08-31-grok-supergrok-to-codex/research/`.
