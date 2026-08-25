# Design — Agent 一键安装与 Codex 多账号认证

## 1. Design Principles

1. **Catalog is policy SSOT**：产品 ID、能力和可执行动作只从现有 Agent Catalog 派生。
2. **Reuse before abstraction**：先适配现有 Tooling / Codex Desktop Installer / Auth Center；只有两个以上真实调用者共享相同机制时才抽通用层。
3. **Closed capabilities**：renderer 只传闭集 ID/action；URL、路径、命令、token 全部由 Rust 侧按产品策略解析。
4. **Ownership before convenience**：Agent-owned 凭据由 Agent 自己持久化；FyAgent-managed OAuth 才进入 FyAgent Auth Center。
5. **Unknown stays unknown**：没有可信状态/来源证据时保持 `unknown/unavailable`，不从目录、文件、网页文案或一次成功下载推断能力。

## 2. Reuse Map

| Need | Reuse first | New code allowed |
| --- | --- | --- |
| CLI detect/version | `services/tooling/{discovery,versions}.rs` | catalog→tool ID adapter only |
| CLI install/update | `services/tooling/lifecycle.rs` + existing anchored command building | current official command updates; Windows closed user-helper adapter if security contract permits |
| Desktop source/download/job/cancel | `codex_desktop/{source,download,jobs,temp,types}.rs` | generalize product-neutral core + per-agent source descriptor |
| Windows/macOS package install | current Codex Desktop platform adapters when package format matches | thin closed package-format adapter only when vendor package type differs; no generic executable/path runner |
| Post-install detect/launch | Codex Desktop runtime + `external_agents` runtime boundary | trusted identity adapters per desktop Agent |
| Managed OAuth | `commands/auth.rs` + existing Auth Center UI | provider-neutral cleanup/cancel only |
| Codex OAuth account store | `proxy/providers/codex_oauth_auth.rs` | schema/identity migration + concurrency correctness |
| Provider-account binding | `ProviderMeta.authBinding` | no new binding schema |
| Codex live auth/config write | `codex_config/{auth,storage}.rs` + Provider/Change Plan locks | effective credential-store resolver and file-mode projection guard |

## 3. P0 Architecture

### 3.1 Agent install/action facade

Evolve the current read-only `agent_install` boundary into a small orchestrator. The module remains the only Agent Catalog install/action facade and delegates to existing domain owners.

Suggested wire shape (names may be adjusted during implementation, semantics may not):

```text
get_agent_install_readiness(agentId)
  -> {
       agentId, installState, updateState,
       releaseId?, // backend-generated opaque source revision for managed packages
       authOwnership, authState, reasonCodes...
     }

start_agent_action({ agentId, action, expectedReleaseId? })
  action = install | update | launch | auth_login | auth_logout | auth_connect_provider
  -> { jobId?, state, reasonCode }

cancel_agent_action({ jobId })
get_agent_action_job({ jobId })
```

No command accepts an installer URL, local path, binary name, shell string, auth token, provider endpoint or validation bypass. For managed-package sources that expose a real version/revision (Codex, TRAE Work, WorkBuddy), `expectedReleaseId` is an opaque value previously produced by the backend; start force-refreshes the source and requires it to still match before creating the download job. QoderWork is the deliberate exception described below: the vendor exposes a versionless `/latest/` artifact alias but no trustworthy remote semantic revision, so FyAgent revalidates that fixed alias and promises only “current latest”, not an invented checked version.

The facade maps canonical catalog ID to one of three adapters:

```text
Agent Catalog
  ├─ CLI Tooling adapter
  │    ├─ claude-code
  │    ├─ grokbuild
  │    └─ opencode
  ├─ Managed package adapter
  │    ├─ codex (existing)
  │    ├─ qoderwork (fixed latest aliases)
  │    ├─ trae-work (official latest API)
  │    └─ workbuddy (official update API)
  └─ Auth action adapter
       ├─ agent_owned
       ├─ provider_owned
       └─ fyagent_managed
```

### 3.2 CLI Tooling adapter

- Do not copy lifecycle command construction into Agent Catalog code. Map:
  - `claude-code -> claude`
  - `grokbuild -> grok`
  - `opencode -> opencode`
- Preserve `probe_tool_installations`, version parsing, canonicalization, anchored update and package-manager fallbacks.
- Treat Agent Catalog as an additional Tooling consumer. Existing Gemini CLI, OpenClaw, Hermes and other Tooling-owned lifecycle surfaces remain independent regression fixtures and must not be routed through the new Catalog façade merely for architectural symmetry.
- Refresh only stale upstream facts. In particular, Claude Code now has an official native Windows installer and WinGet package; this changes source policy but **does not remove FyAgent's formal elevated-Windows process boundary**.
- On formal Windows builds, either:
  1. introduce a narrowly authenticated ordinary-user helper that accepts only a closed `{tool, lifecycleAction/authAction}` enum and internally resolves the fixed command; or
  2. keep the action unavailable.

Option 1 is allowed only if it reuses the frozen Explorer-user context/authenticated helper design and does not become a generic shell bridge. Security review can select option 2 if the proof is incomplete.

The helper protocol, if implemented, returns only closed lifecycle/auth states and sanitized reason codes. It does not return raw child stdout/stderr, environment, browser URL, device code, executable path or command line to the elevated parent/renderer.

### 3.3 Managed package core

The existing Codex Desktop installer is already the repository-wide executable installer policy owner. Refactor by **extracting seams**, not rewriting behavior:

- product source descriptor / release resolver;
- common job state, cancellation and single-flight;
- common bounded streaming download/temp ownership;
- package-format/platform deployment interface;
- product-specific post-install runtime discovery/launch.

Codex behavior stays as golden regression fixture while QoderWork/TRAE Work/WorkBuddy add adapters.

The reusable unit is the **managed-package orchestration and policy**, not an assumption that every Windows product is MSIX. Reuse an existing concrete deployer only when the vendor artifact format matches. A newly evidenced EXE/NSIS/MSI/PKG format may receive a narrow fixed adapter under the same core, with product-owned arguments/identity and the same user-context/cleanup/error discipline; it may not expose an arbitrary process launcher.

Source descriptor rules:

- source belongs to the actual catalog product variant (CN vs global is explicit policy);
- source selection resolves an explicit `(platform, architecture, packageFormat)` branch; marketing claims such as “ARM compatible” are not enough to select an ARM-native artifact;
- fixed first-party domain(s), bounded redirects and metadata;
- redirect targets stay inside an explicit product-owned host allowlist and HTTPS policy; an HTTP downgrade, unknown host or excess hop is rejected before download;
- version/source selection comes from a stable official latest alias or machine update contract, never a current-version literal copied into FyAgent;
- website JS/HTML may be used as research evidence to locate a documented/backend endpoint, but is not itself the runtime resolver unless the vendor explicitly treats it as the stable machine interface;
- no third-party mirror.

Concrete desktop source adapters after 2026-08-25 re-review:

| Product | Runtime source authority | Version semantics | Failure fallback |
| --- | --- | --- | --- |
| QoderWork CN | fixed first-party `/qoder-work-cn/releases/latest/` URLs selected by platform/arch; Windows uses official recommended User-x64 | remote semver unknown; action means install/update the object currently behind the latest alias | official QoderWork download page |
| TRAE Work CN | `api.trae.cn/.../native/version/trae/cn/latest`, bounded same-schema fallback to `api.trae.ai`; select `data.solo` + `region=cn` | parse validated stable-path version / future explicit version field; current evidence `2.3.76922` | official TRAE Work page |
| WorkBuddy | same-origin `/v2/update` with one of three closed platform IDs | API `version/productVersion`; current evidence `5.3.14.36279234` | official WorkBuddy download page |

For TRAE/WorkBuddy, API-provided download URLs are **data, not capabilities by themselves**. Rust accepts them only after HTTPS host, product path prefix, platform/architecture, filename and package-format grammar all match the selected descriptor. Qoder's fixed aliases are code-owned endpoints and do not need a remote locator field.

Qoder has a deliberate source-coherence exception: there is currently no trustworthy remote version/revision comparable with the local application version. The renderer therefore receives no invented `latestVersion`. Starting install/update revalidates the exact fixed alias and proceeds with the semantics “current latest”. This is preferable to using stale docs or HTTP validators as a fake semantic version. TRAE/WorkBuddy retain the stricter checked-release behavior because their official resolver exposes a real versioned source state.

### 3.4 Auth ownership facade

`authOwnership` is capability metadata, not a token source.

| Agent | Ownership | Allowed FyAgent action | Status authority |
| --- | --- | --- | --- |
| Claude Code | `agent_owned` | official CLI login/logout | `claude auth status` JSON + exit code |
| Grok Build | `agent_owned` | official CLI login/logout or first-launch browser flow | only a documented structured CLI status if one exists; otherwise unknown |
| OpenCode | `provider_owned` | open provider connection flow | provider auth list only if a stable structured surface is proven; no global status |
| QoderWork CN | `agent_owned` | launch installed app/login surface | app-owned; unknown unless stable API exists |
| TRAE Work CN | `agent_owned` | launch installed app/login surface | app-owned; unknown unless stable API exists |
| WorkBuddy | `agent_owned` | launch installed app/login surface | app-owned; unknown unless stable API exists |
| Codex | native + `fyagent_managed` coexist | Codex native login remains Codex-owned; managed accounts live in Auth Center | store-aware; see P1 |

FyAgent never reads the other agents' credential files merely to paint a green badge.

## 4. P1 Architecture

### 4.1 Credential identity model

Current store is already multi-account but uses a workspace-level value as map key. Replace the implicit identity with a schema that separates credential and routing scopes:

```text
ManagedCodexCredential {
  credential_id,          // canonical FyAgent key, never workspace id
  subject_identity?,      // only when a stable user claim is verified
  login_label/email?,
  chatgpt_account_id?,    // upstream routing/workspace identity only
  refresh_token,
  authenticated_at,
  requires_reauth,
}
```

Do not name a token claim in code until its stability/semantics are verified against the current OpenAI Codex source or API contract. If no stable user claim is available, generate and persist a random credential UUID at login completion. This may permit duplicate rows for repeated logins, but it is safer than merging distinct people by workspace ID; later deduplication requires positive identity evidence.

### 4.2 One credential SSOT

- `codex_oauth_auth.json` / `CodexOAuthManager` remains the only managed token authority.
- Provider row keeps `authBinding.accountId = credential_id` only.
- Runtime caches hold access tokens only.
- Never persist a copy of the token package into Provider settings merely to switch cards.
- Any refresh path updates the managed store under the existing per-account refresh lock.

### 4.3 Provider binding vs destination

Represent these independently:

```text
credentialSource = native_codex | managed_codex_credential(id) | provider_api_key
upstreamDestination = openai_first_party | configured_provider_endpoint | fyagent_proxy
```

The exact types can reuse existing provider/category/proxy fields; the critical rule is that classification functions may not infer one axis from the other.

### 4.4 Codex native credential projection

OpenAI's public Codex documentation currently exposes `file | keyring | auto`; current open-source code additionally contains an `ephemeral` enum variant. FyAgent currently writes only `auth.json`, so projection becomes store-aware without treating source-only/future variants as a public product promise.

1. Parse effective `cli_auth_credentials_store` according to the installed/current Codex configuration semantics.
2. **File**: reuse existing atomic Codex live write, backup/lock/readback; before overwriting a managed bound credential, reconcile a newer Codex-rotated refresh token only when identity matches.
3. **Any non-file mode** (`keyring`, `auto`, source-visible `ephemeral`, or future/unknown values): do not vendor/reimplement OpenAI's internal credential storage. Unless a stable public Codex command/API can import a selected managed credential, return `native_projection_unavailable` and tell the user to use Codex's own login. Managed account use through FyAgent proxy/takeover remains independent.
4. Never use `auth.json` existence as store selection.

This deliberately rejects a seductive but fragile shortcut from CC Switch v3.20.0: unconditional file overwrite works only under a file-store assumption that is no longer universal.

### 4.5 Concurrency and cancellation

Reuse the current manager locks and add a per-auth-provider operation coordinator:

- one login session per provider/flow ID;
- logout/remove cannot race the same credential refresh/login completion;
- Provider switch cannot publish success until binding/live projection readback agrees;
- cancellation token is retained by backend job state and actually cancels pending HTTP polling;
- all terminal paths remove job/cancel handles.

### 4.6 Migration

Introduce a versioned managed-auth store migration rather than mutating in-memory shape silently:

- read bounded v1;
- create task-owned backup before first v2 write;
- each extant v1 row gets a new credential ID; preserve its current workspace ID as routing metadata;
- update Provider `authBinding` references deterministically where the old row uniquely maps;
- ambiguous/missing binding becomes unbound/requires-user-choice, never guessed;
- authoritative reread after write;
- migration is idempotent and does not delete the backup;
- credentials already lost through the old collision cannot be reconstructed.

## 5. Error / State Semantics

Use closed reason codes. Important terminal states include:

- `source_not_verified`
- `official_page_only`
- `platform_unsupported`
- `interactive_user_unavailable`
- `installed_not_runnable`
- `auth_state_unknown`
- `provider_connection_required`
- `credential_store_unsupported`
- `binding_account_missing`
- `binding_identity_mismatch`
- `operation_conflict`
- `cancelled`

Error text must not contain URL query secrets, device code, access/refresh token, auth JSON body, raw upstream auth response or local secret-bearing path content.

## 6. Test Strategy

Portable tests are the primary contract proof; native HIL is separate evidence.

- Install: closed selector/action, no renderer locator, source-parser bounds, redirects, job cancellation/single-flight, Codex golden regression, post-install readback ambiguity.
- Tooling: existing discovery/anchored update tests remain authoritative; new Agent adapter tests prove exact mapping only.
- Windows: static/target compilation + helper protocol tests; actual Bob/Alice/UAC behavior remains unverified until HIL.
- Auth ownership: each Agent's allowed action/status source, negative secret-file reads, OpenCode provider-owned semantics.
- Codex OAuth: same-workspace distinct users, migration, default/bound/unbound, refresh rotation, timeout/cancel/races, removal during binding, no token DTO/log serialization.
- Credential stores: file positive; every non-file/unknown mode fail-closed for native projection unless backed by an official stable API; no `auth.json` existence heuristic.
- Provider regressions: stale third-party cleanup, official auth preservation, Quick Setup targeted write, takeover, Change Plan.

## 7. Upstream Reuse / Licensing

- CC Switch-derived behavior remains MIT provenance. Port only reviewed behavior/patches and record upstream issue/release/commit references.
- OpenAI Codex is Apache-2.0; its current auth-storage implementation is reference evidence. Do not copy internal monorepo keyring code unless a later review proves that vendoring is preferable to the fail-closed adapter and records license/maintenance cost.
- Vendor install/login commands are execution interfaces, not copied source code.

