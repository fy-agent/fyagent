# Research: current-login-surfaces

- **Query**: Where V2 Agent / Models / Auth start Grok login, logout, and xAI device-code; post-#167 copy/states; trichotomy in presets/seeds/quota/i18n; official Grok CLI status + `~/.grok/auth.json` rule; #141 B7 untouched-validation if Grok Build drafts are touched.
- **Scope**: mixed (internal code + current specs + parent GitHub notes)
- **Date**: 2026-08-31
- **Parent**: `.trellis/tasks/08-31-grok-first-class-iteration`
- **Related**: Discussion #106, Issue #43, closed #107, UAT #141 B7, PR #167 (Agent auth state machine)

## Findings

### 1. Where login / logout / device-code actually start

V2 does **not** have one trichotomy control. The three roads start on three different surfaces.

| Road | Starts on V2? | Owner surface | Start path |
|---|---|---|---|
| Official `grok login` / `grok logout` | Yes — Agent Auth panel only | V2 Agent configuration (detail), not directory compact, not Models | `AgentAuthStatusPanel` → `useAgentAuthSession.start` → `start_agent_auth_session` → `launch_auth_action(GrokBuild)` |
| xAI device-code | No V2 start | v1 Settings Auth Center + Claude/Codex/Claude Desktop provider forms | `XaiOAuthSection` → `useXaiOauth` → `useManagedAuth` → `auth_start_login("xai_oauth")` |
| API Key | V2 Models Quick Setup only | Shared Provider panel for claude/codex/grokbuild; also v1 Grok Build form | `ProviderPanel.requestSave` / fetch / probe; v1 `GrokBuildProviderForm` + third-party presets |

#### V2 Agent — official Grok login/logout

- Directory cards only render **compact** status. Compact mode has no buttons (`src/v2/pages/agents/AgentAuthStatusPanel.tsx:254-259`). Login is not started from the directory row.
- Configuration detail always mounts the full Auth panel above the Models/Skills/MCP/Prompts tabs (`src/v2/pages/agents/AgentConfiguration.tsx:95`).
- Detail buttons call `session.start({ agentId, intent })` (`AgentAuthStatusPanel.tsx:280-291`). Tauri port invokes `start_agent_auth_session` (`src/v2/shared/platform/tauri/feature-ports/agentAuth.ts:34-51`).
- Backend: Grok login launches closed CLI `grok login` and returns **HandoffComplete**; logout runs `grok logout` (`["logout"]`) and also **HandoffComplete** (`src-tauri/src/agent_install/auth_actions.rs:163-169`).
- Session runner immediately terminals as `handoff_complete` + `handoff_only` without a verify loop (`src-tauri/src/agent_install/auth_sessions.rs:541-550`). Claude stays in `awaiting_user` / `verifying` until `claude auth status` proves a state (`auth_sessions.rs:552-606`, `auth_actions.rs:200-224`).
- Codex cannot start a session here: observation `allowed_intents` is empty (`auth_actions.rs:519-529`); `validate_intent` returns `managed_by_auth_center` (`auth_sessions.rs:410-413`).
- Install-readiness **must not** start Auth. `start_agent_action` rejects `auth_login` / `auth_logout` / `auth_connect_provider` as `executor_not_implemented` (`src-tauri/src/agent_install/mod.rs:419-422`). The readiness UI already filters those actions out and only shows install/update/launch (`AgentInstallReadinessSection.tsx:128-131`).

#### V2 Models — no official login, no device-code

- Agent Models section only lists providers and links to `/models?target=…` (`AgentModelsSection.tsx:144-148`, `AgentConfiguration.tsx:47-53`).
- `ProviderPanel` is shared by `claude` / `codex` / `grokbuild` and is **API Key Quick Setup** only (`src/v2/pages/models/Page.tsx:932-941`, reserved id `fyagent-v2-quick-setup-grokbuild` in `quickSetup.ts:11-18`).
- There is no `grok login` button, no Auth Center embed, and no `xai_oauth` account picker on this page. Grep of `src/v2/pages/models` for login/oauth/xai is empty.

#### Auth UI — xAI device-code (v1, not V2)

- Auth Center lives on v1 Settings (`src/components/settings/SettingsPage.tsx:53,312`), section titled `xAI (Grok OAuth)` (`AuthCenterPanel.tsx:75-90`).
- Start: `XaiOAuthSection` button “使用 xAI 登录” / “添加账号或重新登录” (`XaiOAuthSection.tsx:216-232`) → `useXaiOauth()` (`hooks/useXaiOauth.ts:4-6`) → `useManagedAuth("xai_oauth")` → `authApi.authStartLogin` (`useManagedAuth.ts:61-66`).
- Backend command `auth_start_login` with provider `xai_oauth` starts the device-code flow (`src-tauri/src/commands/auth.rs:110-141`). Tokens are stored in FyAgent `xai_oauth_auth.json`, **not** `~/.grok/auth.json` (`src-tauri/src/commands/xai_oauth.rs:17-19`, `src-tauri/src/proxy/providers/xai_oauth_auth.rs:211`).
- Same device-code widget is reused on Claude / Codex / Claude Desktop provider forms when `providerType === "xai_oauth"` (`ProviderForm.tsx:1177-1178`, `ClaudeDesktopProviderForm.tsx:619-624`). **Grok Build’s own form does not mount this widget.**

---

### 2. Exact copy and states after PR #167 (handoff_only vs verified vs fyagent_managed)

Contract test: `tests/v2/pages/agents/AgentAuthStatusPanel.test.tsx`. Copy owner: `AgentAuthStatusPanel.tsx`.

| Agent | Observation kind | Authority | Allowed intents | Idle summary | Idle description | After Login click | Terminal outcome |
|---|---|---|---|---|---|---|---|
| grokbuild | `handoff_only` | `unverified` | `login`, `logout` if CLI detected | **仅支持打开官方认证入口** (`:46-47`) | **FyAgent 只能把操作交给官方应用或 CLI，无法验证最终账号状态。** (`:63-64`) | Stage **已交给官方认证入口** (`:95-96`); reason **已完成入口交接，但没有权威状态可验证。** (`:128-129`) | `handoff_complete` + `handoff_only`. Test forbids **认证结果已验证** (`AgentAuthStatusPanel.test.tsx:161-189`) |
| claude-code | `account` | `verified` when `claude auth status` JSON parses | `login`, `logout` | **已验证登录** / **已验证退出** (`:36-38`) | **状态来自官方结构化命令的回读。** (`:58-59`) | Stage **等待你完成官方认证** (`:89-90`) then **认证结果已验证** (`:93-94`) | `verified` + `verified_logged_in` / `verified_logged_out` (`auth_sessions.rs:650-658`) |
| Codex | `fyagent_managed` | `verified` (destination `auth_center`) | none | **由 FyAgent 认证中心管理** (`:48-49`) | **Codex 托管账号继续由现有认证中心负责，不在此处复制 OAuth 流程。** (`:65-66`) | No 登录 / 连接 Provider buttons (`test:192-218`) | Session start rejected as `managed_by_auth_center`; reason copy **请在现有认证中心管理此账号。** (`:130-131`) — **no navigation button** |

Grok observation builder (`auth_actions.rs:69-86`):

- CLI available → `handoff_only` + intents `[Login, Logout]` + reason `handoff_only`.
- CLI missing → `unavailable` + `auth_observer_unavailable`. Summary **当前无法读取认证状态** (`AgentAuthStatusPanel.tsx:50-51`); description **认证观察器不可用；不会读取厂商凭据文件或推断登录状态。** (`:67-68`).

Compact directory line is only `认证：{summary}` (`:257`). Grok therefore shows `认证：仅支持打开官方认证入口`, never “已登录”.

Backend table matches spec: “Grok or desktop Auth entry opens successfully → `handoff_complete` + `handoff_only`; never verified” (`.trellis/spec/backend/external-agent-p0.md:535`).

---

### 3. How the three roads are distinguished today

They are distinguished by **app + category + `providerType`**, not by a shared V2 trichotomy enum.

#### Presets

**Grok Build** (`src/config/grokBuildProviderPresets.ts`):

- Official: `grokBuildOfficialPreset` name `"Grok Official"`, `category: "official"`, empty `auth` + empty `config` (`:44-53`). File comment: official OAuth is *not* a preset; official state is this empty seed (`:8-11`, `:41-42`).
- Third-party / aggregator: `grokAuth()` = `{ OPENAI_API_KEY: "" }` (`:58`). Includes a preset literally named `"xAI (Grok)"` hitting `https://api.x.ai/v1` with **API Key**, `category: "third_party"` (`:427-437`). This is **not** device-code.
- **No `providerType: "xai_oauth"`** on any Grok Build preset.

**Claude / Codex / Claude Desktop** (device-code lives here):

- Claude: `"xAI (Grok)"` + `providerType: "xai_oauth"` + `requiresOAuth: true` (`src/config/claudeProviderPresets.ts:1227-1244`).
- Codex: `"xAI (Grok) OAuth"` + `providerType: "xai_oauth"` (`src/config/codexProviderPresets.ts:1415-1436`). Comment: proxy injects token; base_url/empty auth are snapshots.
- Claude Desktop: `"xAI (Grok)"` + `providerType: "xai_oauth"` (`src/config/claudeDesktopProviderPresets.ts:745+`).

v1 Grok Official copy is only shown when `category === "official"` (`GrokBuildProviderForm.tsx:436-443`). Third-party presets do not show the `grok login` lecture (`tests/components/GrokBuildProviderForm.test.tsx:27-63`).

#### Seed providers

`src-tauri/src/database/dao/providers_seed.rs:74-83`:

- id `grokbuild-official`
- name `"Grok Official"`
- website `https://x.ai/grok`
- `settings_config_json`: `{"config":""}` — empty config so Grok CLI falls back to its own login
- Seed test locks the empty-config contract (`:109-118`)
- `ensure_grokbuild_official_provider` keeps this row present (`src-tauri/src/commands/provider.rs:865`)

No seed exists for xAI device-code on the grokbuild app. Device-code accounts live in `xai_oauth_auth.json`.

#### Quota footers

`ProviderCard.tsx:229-232,475-502` picks footer by type:

1. `meta.providerType === xai_oauth` → `XaiOauthQuotaFooter` → `appIdForExpiredHint="xai_oauth"` (`XaiOauthQuotaFooter.tsx:36`)
2. else official grokbuild → `SubscriptionQuotaFooter` → remaps appId to `"grok"` so copy says `grok login` (`SubscriptionQuotaFooter.tsx:442-443`)
3. else usage-script / API Key path

Expired-hint switch (`SubscriptionQuotaFooter.tsx:80-90`):

- `grok` / `grokbuild` → `subscription.grokOfficialExpiredHint`
- `xai_oauth` → `subscription.xaiOauthExpiredHint`
- else generic `subscription.expiredHint` with `{tool}`

Tests lock the split: Official Grok expiry mentions `grok login`; xAI expiry mentions Auth Center and never `grok login` (`tests/components/SubscriptionQuotaFooter.test.tsx`, `tests/components/XaiOauthQuotaFooter.test.tsx`).

**Caveat:** official grokbuild quota **does** read `~/.grok/auth.json` to call grok.com billing (`src-tauri/src/services/subscription_grok.rs:1-14,38-40`). xAI OAuth quota uses the managed token and explicitly not that file (`xai_oauth.rs:17-19`). This is a quota owner, not an Auth-observation owner.

#### i18n (all four locales)

| Key | en | zh |
|---|---|---|
| `providerForm.grokOfficialHint` | Grok Official uses an empty config. After you save, run `grok login` in a terminal. FyAgent does not log in for you and does not write ~/.grok/auth.json. | Grok Official 使用空配置。保存后请在终端运行 `grok login`。FyAgent 不会代为登录，也不会写入 ~/.grok/auth.json。 |
| `subscription.grokOfficialExpiredHint` | Run `grok login` in a terminal to refresh this login. | 请在终端运行 `grok login` 以刷新此登录。 |
| `subscription.xaiOauthExpiredHint` | Re-authenticate this xAI account in Auth Center. | 请到认证中心重新登录此 xAI 账号。 |
| `settings.authCenter.xaiOauthDescription` | Manage xAI / Grok accounts | 管理 xAI / Grok 账号 |
| `xaiOauth.login` | Sign in with xAI | 使用 xAI 登录 |
| `providerForm.officialHint` (Claude/generic) | Official provider uses browser login, no API Key needed | 官方供应商使用浏览器登录，无需配置 API Key |

ja / zh-TW have the same trichotomy split (`src/i18n/locales/{en,zh,ja,zh-TW}.json`).

V2 Agent / Models copy is **hardcoded Chinese**, not these i18n keys. Grok’s Agent panel never says `grok login`; it only says “打开官方认证入口”.

---

### 4. Official Grok CLI status surface — and the auth.json rule

**No reviewed Grok auth-status observer exists in this repo.**

What exists:

- Launch only: `grok login` / `grok logout` (`auth_actions.rs:163-169`).
- Availability probe: `ensure_tool_available(GROK_TOOL_ID)` where `GROK_TOOL_ID = "grok"` (`auth_actions.rs:70`, `src-tauri/src/agent_install/cli.rs:7`).
- Archived review: official CLI docs list `grok login`, `grok logout`, and `grok inspect --json`. Inspect is **project configuration**, not an auth-status contract (`.trellis/tasks/archive/2026-08/08-29-agent-auth-verification-state-machine/research/official-auth-surfaces.md:27-37`). `grok inspect` is not called anywhere in product code.

Claude contrast: bounded `claude auth status` JSON with allowlisted fields (`auth_actions.rs:36-44,200-224`). OpenCode contrast: `opencode auth list` (`:227-246`). Grok has neither.

**Reading `~/.grok/auth.json` to prove login is forbidden by current spec.**

- `.trellis/spec/backend/external-agent-p0.md:508-509`: Grok Build has no reviewed structured status, so official login/logout ends in `handoff_complete`, not `verified`.
- Same spec `:518-519`: “Never read vendor token files, Keychain, browser cookies, or credential-store entries to infer state.”
- Parent decision notes: `#43` / `#106` — do not read `~/.grok/auth.json` to fake logged-in (`.trellis/tasks/08-31-grok-first-class-iteration/research/github-decision-43.md:7-8`, `github-decision-106.md:7`).
- Product copy already tells the user FyAgent will not write that file (`providerForm.grokOfficialHint`).
- Unavailable-observation copy: will not read vendor credential files (`AgentAuthStatusPanel.tsx:67-68`).

**Not the same rule as quota.** `subscription_grok.rs` already reads `~/.grok/auth.json` for SuperGrok credit display. That path must not be reused as Auth observation / “已登录” proof.

---

### 5. #141 B7 — untouched Grok Build model-draft validation

B7 (UAT #141): empty drafts must not show submit/validation errors on route mount; fetch / probe / save own validation; corrected paths clear it.

Classification on 2026-08-30: **fixed (automated)** for the shared Models page (`.trellis/tasks/archive/2026-08/08-29-frontend-reliability-architecture/research/uat-current-main-mapping.md:28`). Parent iteration note: if Grok Build drafts are touched, re-verify B7 on latest main and mark `fixed` / `still applies` / `not touched` (`.trellis/tasks/08-31-grok-first-class-iteration/research/github-decision-141.md`).

Current Grok Build draft surface is the **shared** `ProviderPanel` (`Page.tsx:932+`), not a Grok-only draft widget.

Untouched behavior (still true on current main):

- `errors` initializes to `{}` (`Page.tsx:956`). No `useEffect` runs `validateQuickSetup` on mount.
- `validateQuickSetup` only runs inside `requestSave` (`:1079-1102`). Empty name/url/key/model then become “请输入配置名称 / 请输入不含账号信息的 HTTP(S) 地址 / 请输入 API Key / 请输入模型 ID” (`quickSetup.ts:89-93`).
- Fetch validates URL + API Key only after 拉取模型 (`Page.tsx:1005-1018`).
- Probe validates URL + API Key only after connectivity prepare (`:1057-1076`).
- Field errors render only when the corresponding `errors.*` is set (`:1405-1413`, `:1446-1453`).
- Dirty tracking (`useModelsDraftCommit`) does not validate (`modelsShared.tsx:27-50`).

There is **no Grok-specific B7 test**. `tests/v2/pages/models/Page.test.tsx` mentions Grok Build only in rail order (`:261-265`). Browser spec `tests/v2-browser/agents-models.spec.ts` likewise only asserts catalog order. WorkBuddy/OpenCode empty-draft errors are save-gated (`Page.tsx:299`, `OpenCodeModelsPanel.tsx:347-348`), same pattern.

If this task only changes Agent Auth copy / Auth Center handoff and does **not** edit `ProviderPanel` / `quickSetup.ts` / Grok Build form draft fields, B7 is **not touched**. If those drafts are edited, re-check: open `/models?target=grokbuild` with empty fields and assert no `role="alert"` / `fy-control-field-error` until 保存 / 拉取模型 / probe.

v1 `GrokBuildProviderForm` TOML editor uses `showValidation={false}` (`GrokBuildProviderForm.tsx:572`); malformed TOML error is shown only when `rawConfigError` is set (`:575-582`). Official category hides the whole config block (`:449`).

---

### Files Found

| File Path | Description |
|---|---|
| `src/v2/pages/agents/AgentAuthStatusPanel.tsx` | V2 copy + login/logout buttons |
| `src/v2/pages/agents/useAgentAuthSession.ts` | Session poll; terminals include `handoff_complete` |
| `src/v2/pages/agents/AgentConfiguration.tsx` | Detail Auth panel mount |
| `src/v2/pages/agents/AgentDirectory.tsx` | Compact auth slot only |
| `src/v2/pages/agents/AgentModelsSection.tsx` | Provider list; no login |
| `src/v2/pages/models/Page.tsx` | Shared API Key Quick Setup for grokbuild |
| `src/v2/pages/models/quickSetup.ts` | Reserved id + save-time validation |
| `src/v2/shared/platform/tauri/feature-ports/agentAuth.ts` | `start_agent_auth_session` IPC |
| `src-tauri/src/agent_install/auth_actions.rs` | Grok handoff vs Claude observe vs Codex managed |
| `src-tauri/src/agent_install/auth_sessions.rs` | HandoffComplete short-circuit |
| `src-tauri/src/commands/auth.rs` | `auth_start_login` device-code |
| `src/components/settings/AuthCenterPanel.tsx` | v1 xAI device-code section |
| `src/components/providers/forms/XaiOAuthSection.tsx` | Device-code UI |
| `src/config/grokBuildProviderPresets.ts` | Official empty vs API Key presets |
| `src-tauri/src/database/dao/providers_seed.rs` | `grokbuild-official` seed |
| `src/components/SubscriptionQuotaFooter.tsx` | Official vs xAI expiry copy |
| `src-tauri/src/services/subscription_grok.rs` | Reads `~/.grok/auth.json` for quota only |
| `.trellis/spec/backend/external-agent-p0.md` | Auth contract + no credential-file inference |

### Related Specs

- `.trellis/spec/backend/external-agent-p0.md` — observation kinds, Grok handoff, never read vendor tokens
- `.trellis/spec/frontend/v2-agent-models.md` — Grok Quick Setup reserved id / live `~/.grok/config.toml` (models, not login)
- `.trellis/spec/backend/windows-runtime-security.md:498-511` — auth observation/session; helper must not return device code / credential paths

## Caveats / Not Found

- `python ./.trellis/scripts/task.py current --source` returned no active task; this note was written to the path the caller named.
- No `grok auth status` / structured Grok login observer in product code. `grok inspect --json` is documented as non-auth and unused.
- V2 has no Auth Center route and no button from the Codex façade to Settings.
- V2 Models cannot express official Grok or xAI device-code; it only writes `fyagent-v2-quick-setup-grokbuild` with an API Key.
- Official grokbuild quota still reads `~/.grok/auth.json`. That is not license to treat the file as login proof.
- No dedicated Grok B7 automated test; B7 is the shared ProviderPanel contract.

## Confirmed facts

1. Official Grok login/logout on V2 starts only from Agent configuration Auth buttons and ends `handoff_only` / `handoff_complete`. Opening the CLI is not “已登录”.
2. xAI device-code starts only from v1 Auth Center / Claude·Codex·Claude Desktop `xai_oauth` forms via `auth_start_login`. Grok Build presets and V2 Models do not start it.
3. API Key is the third road: Grok Build third-party presets + V2 Quick Setup `fyagent-v2-quick-setup-grokbuild`. The preset named `"xAI (Grok)"` on Grok Build is API Key to `api.x.ai`, not device-code.
4. Current spec forbids reading `~/.grok/auth.json` to prove login. No official Grok status command is reviewed or implemented.
5. B7 is save/fetch/probe-gated on the shared Models panel. Empty Grok Build drafts stay silent on mount unless that panel is edited.

## Reuse owners

| Need | Reuse, do not rewrite |
|---|---|
| Official `grok login` / `logout` | `launch_auth_action` + `AgentAuthStatusPanel` + `start_agent_auth_session` |
| Device-code | `XaiOAuthSection` / `useManagedAuth("xai_oauth")` / `auth_start_login` / `XaiOAuthManager` |
| Official empty provider | `grokbuild-official` seed + `grokBuildOfficialPreset` + `providerForm.grokOfficialHint` |
| Expiry copy split | `getSubscriptionExpiredHintKey` + four-locale keys already tested |
| Codex Auth | Keep `fyagent_managed` → existing Auth Center; do not add a second OAuth on Agent |
| Claude verify loop | Keep `claude auth status`; do not copy it onto Grok |

## Recommended MVP changes

Stay on copy + entry wiring. Do not add a Grok status parser.

1. On V2 Agent Grok Auth panel, name the official road: next step is terminal `grok login` / `grok logout`. Keep terminal stage `handoff_complete`. Do not say 已验证 / 已登录.
2. Point Codex (already) and Grok’s **device-code** next-step at Auth Center xAI section. Do not start device-code from Agent Auth. A deep-link/button to v1 Settings Auth Center is enough if in scope; do not reimplement OAuth.
3. Keep API Key on V2 Models / third-party presets. Do not show `grok login` on Quick Setup or on `"xAI (Grok)"` API Key presets.
4. If Agent copy mentions expiry, reuse `grokOfficialExpiredHint` vs `xaiOauthExpiredHint`; do not send xAI expiry to `grok login`.
5. If Grok Build drafts are not required to tell the three roads apart, leave `ProviderPanel` / `quickSetup.ts` alone so B7 stays **not touched**.

## Must stay out of scope

- SuperGrok write-into Claude / Desktop / Codex / WorkBuddy（sibling tasks; this file still owns login facts only）
- New OAuth / token relay / inventing `grok auth status`
- Reading or writing `~/.grok/auth.json` to claim verified login
- Promoting quota-file-read into an Auth observer
- Changing Claude `verified` or Codex `fyagent_managed` contracts
- Grok install/upgrade (#31 / #32), V2 quota dashboard, Claude targeting
- Closing umbrella #43
- B7 rewrite unless Grok Build draft validation is actually edited
