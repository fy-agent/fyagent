# Research: OpenCode Desktop Auth Capability Matrix

- **Query**: Freeze no-CLI Desktop capability, official Provider Connect vs FyAgent store projection, restart/reload unknown until HIL, independent Credential Session vs Codex/Proxy lineage, license (no cockpit-tools copy).
- **Scope**: mixed
- **Date**: 2026-09-03

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src-tauri/src/agent_install/auth_actions.rs` | Current PATH-CLI observer/launcher |
| `src-tauri/src/agent_install/auth_sessions.rs` | OpenCode cannot bind inventory targets |
| `src-tauri/src/agent_install/lifecycle_policy.rs` | Desktop-only lifecycle |
| `src-tauri/src/commands/managed_auth.rs` | Connection/login IPC currently `unavailable` |
| `src-tauri/src/services/managed_auth/core.rs` | `OpencodeProvider` purpose, `Opencode` consumer/refresh owner |
| `src/v2/pages/agents/AgentAuthStatusPanel.tsx` | Routes OpenCode to `/auth` |
| `src/v2/shared/features/managed-auth.ts` | `pendingRestart`, `restart` action, consumer `opencode` |
| `.trellis/spec/backend/managed-auth.md` | Identity vs Credential Session; native-owned sessions not Proxy-refreshed |
| `.trellis/spec/frontend/v2-managed-auth.md` | Closed consumers/providers; restart copy |
| `.trellis/tasks/09-03-unified-agent-auth-management/design.md` | §9–10.3 session policy and OpenCode adapter paths |
| `.trellis/tasks/09-03-unified-agent-auth-management/research/auth-source-and-reuse-review.md` | License + independent session freeze |
| Official pin `anomalyco/opencode@b578b726` | Auth schema, Desktop Connect, sidecar |

### Capability matrix

Legend: **Have** = exists in current FyAgent tree. **Official** = exists in pinned OpenCode. **HIL** = not proven on current stable Desktop. **Out** = task/product forbids.

| Capability | Current FyAgent | Official OpenCode (pin) | This slice freeze |
|---|---|---|---|
| Observe providers with Desktop installed and **no PATH `opencode` CLI** | **Absent.** `observe_opencode_providers` runs `opencode auth list` via Tooling locate. Missing binary → `AuthObserverUnavailable`. | Desktop Connect + `Auth.all` read `Global.Path.data/auth.json` inside the app/sidecar. CLI is optional. | Desktop-first observe must not use PATH CLI. |
| Connect provider with no PATH CLI | **Absent.** Connect launches terminal `opencode auth login`. | `dialog-connect-provider.tsx` → server SDK `integration.connect.key` / `integration.oauth.*` on the **private** sidecar. | Path A: launch official Desktop Connect (handoff). Path B: write `auth.json` as projection. **Out:** scan sidecar port/password. |
| Disconnect / remove one provider | CLI `opencode auth logout` + provider-set verify | `Auth.remove(providerID)` 0600 rewrite | Projection removes one known key; CLI optional only. |
| Bind selected Desktop **install** via inventory `i1:`/`c1:`/`r1:` | Lifecycle inventory **Have**. Auth sessions **reject** OpenCode inventory triplets (`TargetChanged`). | N/A (app is the server) | Inventory binds **launch target** for Connect handoff. Credential path is **user XDG data**, not the `.app`/`.exe` path. |
| Resolve data dir from user context without port scan | `get_opencode_data_dir()` uses frozen `get_home_dir()`; Windows ignores `XDG_DATA_HOME`. Unused by Auth. | `path.join(xdgData, "opencode")`; `xdgData = XDG_DATA_HOME \|\| ~/.local/share`. Windows default is `%USERPROFILE%\.local\share\opencode`, not `%LOCALAPPDATA%`. | Use frozen Shell/home user + official XDG rule. **Out:** sidecar port scan, Electron `userData` as auth root. |
| Read/write official `auth.json` | **No production R/W.** Agent Auth spec currently forbids vendor token-file reads **in that façade**. | `Auth` owner is the file. | Managed Auth consumer adapter is the projection owner; Agent Auth façade stays CLI-optional/summary. |
| OAuth schema `{type,refresh,access,expires,accountId?}` | Not parsed | **Have**, plus `enterpriseUrl?` | Validate this shape for oauth entries. Extra official fields on oauth (`enterpriseUrl`) are first-party. |
| API schema `{type,key}` | Not parsed | **Have**, plus `metadata?` | Validate; preserve `metadata` on RMW. |
| `wellknown` entries | Parser accepts `wellknown` **label type** from CLI text; never reads file | `{type,key,token}` | Preserve on RMW; not a FyAgent-managed closed provider unless separately listed. |
| Permissions `0600` | N/A (no writer) | `writeJson(..., 0o600)` then chmod | Required on projection writes. Pin’s write is in-place, not atomic rename. |
| Unknown provider / undecodable entry preservation | N/A | Official `Auth.all` **drops** undecodable values before `set` rewrite | FyAgent projection requirement (parent PRD): do **not** drop unknown keys. This is **stricter** than official `Auth.set` at the pin. |
| Env-var / config-sourced providers | CLI list prints a second “Environment” block; FyAgent parser ignores it | Env keys are not `auth.json` entries | Must not treat env providers as file rows or delete them by rewriting `auth.json`. |
| Official Provider Connect as public HTTP from FyAgent | **Out** in current code (no sidecar client) | Routes exist on sidecar; password + random port + `cors: ["oc://renderer"]` | **Out:** drive Connect by guessing port/password. In: launch Desktop / documented handoff. |
| In-process reload after Connect | N/A | UI `refreshProviders()`; legacy `instance.dispose()` | Proven only for **in-app** Connect. |
| Hot reload after **external** `auth.json` write | N/A | `Auth.all` re-reads file per call; no file-watch in Auth module | **Unknown until HIL.** UI already has `pendingRestart` / reason `pending_restart`. Do not claim live Desktop pickup from source reading. |
| Independent Credential Session for OpenAI/xAI → OpenCode | Enums **Have**; login/connection IPC **unavailable** | OpenCode refresh happens inside OpenCode after oauth entry exists | Default: new session `purpose=opencode_provider`, `consumer=opencode`, `refresh_owner=opencode`. **Do not** copy Codex/Proxy refresh token lineage into `auth.json`. |
| Shared refresh lineage / “one login, many apps” token copy | Spec forbids `shared` owner | OpenCode will refresh any oauth `refresh` it holds | Copying Proxy/Codex refresh into OpenCode makes OpenCode a second rotator. **Out.** |
| cockpit-tools OpenCode projector | Not in tree | N/A | **Out.** CC BY-NC-SA 4.0; parent freeze: behavior reference only. |
| License for official schema/path | FyAgent PolyForm NC + MIT CC Switch | OpenCode **MIT** | Prefer current FyAgent owners + official OpenCode MIT facts. Record pin if copying snippets. |

### Official Connect vs FyAgent projection (two paths)

Parent `design.md` §10.3, restated as facts:

**Path A — OpenCode-managed Connect**

- User completes Provider Connect inside Desktop (or TUI `/connect` / CLI `opencode auth login` if that surface exists).
- FyAgent observes resulting provider list/metadata.
- Refresh owner is OpenCode from the moment OpenCode writes the oauth entry.
- FyAgent does not hold that refresh lineage.

**Path B — FyAgent-initiated independent session, then project**

- FyAgent creates a **new** Credential Session for `consumer=opencode` (same `ManagedIdentity` as Codex/Proxy is allowed; **new** rotating grant).
- Projection writes one provider key into official `auth.json` (oauth or api schema), RMW, 0600, readback.
- After successful readback, `refresh_owner` becomes `opencode`; FyAgent stops refreshing that session.
- Proxy `purpose=proxy_upstream` sessions stay `refresh_owner=fyagent` and are never the OpenCode file contents.

Current tree implements **neither** path end-to-end. UI already labels Path B’s manager as “由 OpenCode 自动续期” (`src/v2/pages/auth/presentation.ts`). IPC `managed_auth_apply_connection_action` returns `unavailable`.

### no-CLI Desktop: what is true today

```text
Desktop inventory/launch  : Have (lifecycle Desktop-only)
CLI lifecycle install     : SurfaceNotSupported
Auth observe/connect      : Requires PATH `opencode` via Tooling
Managed Auth connections  : unavailable
auth.json projection      : Not implemented
sidecar client            : Not implemented (and out of scope to scan)
```

Therefore “Desktop installed, CLI missing” currently yields Agent Auth `unavailable`/`unknown`, not a store-backed provider list.

### Restart / reload (HIL gate)

Already modeled in V2 wire (`pendingRestart`, action `restart`, reason `pending_restart`). Backend connection commands are unavailable, so nothing sets those flags for OpenCode yet.

Official in-app Connect recycles the server instance. External writer behavior is **not** frozen from source. Community/docs mention full quit/relaunch for Desktop issues. **Capability stays `pending_restart` / `unsupported` until macOS and Windows stable Desktop HIL records pickup vs restart.**

### Independent Credential Session vs Codex/Proxy lineage

From `.trellis/spec/backend/managed-auth.md` and parent research §5:

```text
ManagedIdentity     = issuer + subject/tenant + display
CredentialSession   = one grant + one rotating refresh lineage + one purpose/consumer
RefreshOwner        = fyagent | codex_native | grok_native | opencode | unavailable
```

Same OpenAI/xAI identity on Codex, Proxy, and OpenCode ⇒ **three sessions**, not one token copied three times. OpenCode oauth `refresh` field is a lineage OpenCode will rotate. Putting the Proxy/Codex refresh string into that field is sharing a lineage.

### License

| Source | License | Use in this slice |
|---|---|---|
| Current FyAgent `agent_install`, `opencode_config`, `managed_auth`, inventory | Project LICENSE (PolyForm NC + MIT CC Switch-derived) | Primary implementation surface |
| `anomalyco/opencode` pin `b578b726` | MIT | Schema/path/Connect facts; copy snippets only with copyright notice |
| OpenAI Codex / xAI Grok | Apache-2.0 | Not OpenCode store format; do not reuse their `auth.json` shape here |
| `jlcodes99/cockpit-tools` | CC BY-NC-SA 4.0 | **No copy**, no vendor, no binary embed |

### Related Specs

- `.trellis/spec/backend/managed-auth.md`
- `.trellis/spec/frontend/v2-managed-auth.md`
- `.trellis/spec/backend/external-agent-auth.md`
- `.trellis/spec/backend/external-agent-lifecycle.md`

### External References

- Pin: https://github.com/anomalyco/opencode/tree/b578b7261fc9ec4917fe272df5cc4bd8a056cd5d
- Docs: https://opencode.ai/docs/troubleshooting/
- Parent: `.trellis/tasks/09-03-unified-agent-auth-management/research/auth-source-and-reuse-review.md`

## Caveats / Not Found

- Windows dual-path rumor (`%LOCALAPPDATA%\opencode\auth.json` vs `%USERPROFILE%\.local\share\opencode\auth.json`) is **not** resolved without HIL.
- Whether current stable Desktop still uses sidecar v1 (`listen(0)` + UUID) vs `OPENCODE_SIDECAR_V2` background CLI is source-conditional in `index.ts`; production default in the pin is v1 unless `OPENCODE_SIDECAR_V2=1`.
- Closed FyAgent-managed provider IDs (openai / xai / github_copilot vs OpenCode’s `github-copilot` key) are not mapped in current code.
- Native HIL for connect, refresh writeback, disconnect, external change, and restart is explicitly open in parent research §7 item 4.
