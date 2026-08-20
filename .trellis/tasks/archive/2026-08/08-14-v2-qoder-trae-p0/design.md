# 技术设计：QoderWork / TRAE Work P0 与共享 Catalog

## 1. Boundaries and Data Flow

```text
V2 pages
  -> typed feature ports
  -> strict Tauri runtime adapters
  -> narrow commands
     -> static catalog v3
     -> external-agent runtime adapter
     -> existing SkillService + target adapters
     -> Qoder safe document adapter
     -> TRAE endpoint/MCP validators
```

静态厂商事实、本机探测事实、用户操作结果分别建模。所有 renderer 输入先通过封闭 DTO 验证；最终路径、进程目标、DNS 地址和文件写入由 Rust 决定。

## 2. Shared Catalog UI

- 新增 `src/v2/shared/ui/catalog/`，拥有 `CatalogMasterDetail`、`CatalogRail`、`CatalogListItem`、`CatalogDetail`、`BrandIconFrame` 和唯一 catalog stylesheet。
- Agent/Models 页面只提供 rail items 与 detail 内容，不再拥有目录列、row、frame 或断点 CSS。
- CSS tokens 固定：rail `clamp(220px, 24vw, 268px)`、gap 14px、row 56px、list frame/artwork 36/28px、detail frame/artwork 64/48px、stack 760px。
- `scrollbar-gutter: stable` 放在共享 feature viewport；route/target 切换只允许 detail opacity/小幅 y 动效，不改变 geometry。
- typed brand metadata 是单一资产映射，页面不得根据 Agent ID 追加 class。

## 3. Catalog v3 and Runtime Types

```ts
type CapabilityId =
  | "product.open" | "app.detect" | "app.launch"
  | "skills.read" | "skills.write"
  | "hooks.read" | "hooks.write"
  | "models.validate" | "models.write"
  | "mcp.validate" | "mcp.write";

type DeclaredCapability = {
  id: CapabilityId;
  mode: "direct" | "assisted" | "unsupported" | "unverified";
  reasonCode: string;
  evidenceIds: string[];
};

type AgentCatalogResultV3 = {
  contractVersion: 3;
  reviewedAt: string;
  agents: Array<{
    id: AgentId;
    variantId: string;
    displayName: string;
    description: string;
    officialLinks: AgentOfficialLink[];
    capabilities: DeclaredCapability[];
  }>;
};
```

Runtime command returns `detected/running: boolean | null`, optional version and install source, plus per-capability `available | assisted | unavailable | unverified | blocked_by_version | probe_failed`. Catalog parsing is exact and rejects v2, future versions, unknown enum values, duplicate IDs and invalid links.

`launch_external_agent` accepts only `AgentId` and `home | skills | hooks | models | mcp`. Current P0 contains no guessed executable identity. An adapter without trusted identity returns a controlled unverified/unsupported result and never starts a process.

## 4. Skills Domain

```rust
pub enum SkillTargetId {
    Claude, Codex, Gemini, GrokBuild, OpenCode, Hermes,
    QoderWork, TraeWork,
}
```

- Existing `AppType` adapters cover only the original six targets.
- `SkillApps` retains boolean wire compatibility and adds default-false QoderWork/TRAE Work fields.
- Skill target and MCP target constants are separate in renderer and backend tests.
- Qoder path is `<trusted-home>/.qoderwork/skills`; TRAE CN path is `<trusted-home>/.trae-cn/skills` on all supported OSes.
- Existing `SkillService` remains the owner of archive validation, copy, conflict handling, SSOT and reread. Target adapters supply only trusted destination semantics.
- Observation never creates target directories. User-initiated sync may create them after ancestor/leaf validation.

## 5. Safe Qoder Document

Commands:

```text
get_qoderwork_hooks()
save_qoderwork_hooks({ request })
```

Snapshot contains only opaque revision, exists, structured groups, restartRequired and supported-structure state. Raw unknown values never cross IPC.

Read sequence: resolve trusted home and fixed path; pin/validate ancestors; acquire document lock; bound read to 2 MiB; require JSON object; project supported hooks; compute process-local HMAC revision; return sanitized snapshot.

Save sequence: validate group/event/matcher/command/timeout bounds; acquire lock; reread and compare expected revision; preserve raw top-level object; reject unsupported hook structures; replace only hooks; require explicit overwrite token only for a freshly reviewed destructive conflict; create backup; write same-directory temp; flush/sync; confirm target identity; atomic replace; reread and return new snapshot.

Concurrent revision drift never writes. Tokens bind agent/path/revision/request digest, have a short TTL and are single-use. Validation never parses shell effects or executes commands.

## 6. TRAE Model Probe

Commands:

```text
validate_traework_model_config({ request })
test_traework_model_endpoint({ requestId, request })
cancel_traework_model_endpoint({ requestId })
```

`requestId` is canonical UUID syntax. Active requests are stored in a feature-local cancellation map; duplicates reject, terminal paths remove entries, and cancellation participates in DNS/connect/TLS/read through `tokio::select`.

Validation rejects non-HTTP(S), userinfo, fragment, uncontrolled query, malformed host, control characters, credential collision and unsafe header overrides. API Key is held by a redacted Rust type and never appears in Debug/serde errors.

Network policy: HTTPS by default; loopback HTTP/private networks require separate consent; metadata, multicast, unspecified, broadcast and link-local fail closed; resolve all A/AAAA and reject mixed unsafe answers; pin approved address; zero redirects; connect timeout 3s; total deadline 10s; body at most 1 MiB; compression disabled.

Proxy resolution reuses existing `installer_proxy_configuration()`. Explicit/system proxy is permitted only when the transport can tunnel to the pinned socket while preserving original Host/SNI. Otherwise return `PROXY_DNS_PIN_UNSUPPORTED`; never silently fall back to direct.

Result returns only terminal classification, duration bucket/status class and normalized non-secret summary. It never returns body, headers, query, complete host error or secret.

## 7. MCP Validation

Input must be an object with `mcpServers`. Each server is exactly one of:

```ts
type StdioServer = { command: string; args?: string[]; env?: Record<string, string> };
type HttpServer = { url: string; headers?: Record<string, string> };
```

Reject mixed transports, prototype-pollution keys, control characters, invalid arrays/maps and bounded-input violations. Stdio checks a single executable via platform resolver or a fixed path without shell composition and without execution. HTTP applies URL/address classification but performs no connection. Findings include only server ID, transport, controlled reason codes, executable availability and `hasSecrets`; values are omitted.

The renderer may generate a redacted template with secret values removed/replaced. Full configuration remains component-local and is not put in query state, persistence or default clipboard.

## 8. Frontend State and Copy

- Static catalog and sanitized runtime status may use React Query.
- Hook command editing, API Key and full MCP env/header data use local component state and mutation-only calls.
- Success, failure, cancel, timeout, target change, route leave and unmount clear sensitive state.
- Agent detail owns status, Skills, Hooks/MCP entry points. Models owns the TRAE preflight form; Qoder Models shows truthful built-in-model guidance and links to supported capabilities.
- Browser adapter returns native-only/unverified results. Rich behavior exists only in focused fake-Tauri tests.

## 9. Permission and Error Contract

Commands are grouped into narrow observe, launch, qoder-write and endpoint-probe permissions. The main local app webview receives only required commands; remote content receives none. No generic filesystem, shell, arbitrary path or arbitrary executable command is introduced.

Closed errors include catalog/runtime failures, unverified launch, invalid Qoder settings, concurrent modification, overwrite required, invalid/private TRAE URL, proxy DNS-pin unsupported, auth/model/network rejection, invalid MCP config, missing command, secret presence and cancellation. DTOs/logs carry only operation ID, Agent ID, reason code and duration/status class.

## 10. Compatibility, Evidence and Rollback

- Catalog Rust and renderer adapters move atomically from v2 to v3; no legacy guessing fallback.
- `SkillApps` is additive and rollback-safe for old readers.
- Shared UI can be reverted independently from backend capability batches.
- Qoder writes are backup-first; unknown rollback authority returns partial/unknown rather than success.
- TRAE model/MCP work does not write vendor storage, so rollback removes only FyAgent validators/UI.
- HIL-gated runtime, Skill recognition and Hook effectiveness remain `unverified`. Automated validators owned entirely by FyAgent may be marked direct/available after tests pass.

