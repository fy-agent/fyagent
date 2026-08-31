# Technical Design

## 1. Design principles

1. **产品策略和运行证据分离。**
   “某产品允许 install/update/launch”是稳定产品策略；“当前机器是否安装、版本、路径是否可写”是运行证据。两者必须同时满足才产生 action。

2. **Catalog order 不是运行排序。**
   Catalog 保持跨页面稳定的 canonical order；Agent Directory 只在扫描完成后投影一个运行时顺序。

3. **隐藏按钮不是权限边界。**
   renderer 只显示 backend `allowedActions`，但 backend 仍必须在 action dispatcher 最前面拒绝不支持的 action/surface。

4. **产品 ID 不等于物理安装形态。**
   `claude-code` 与 `opencode` 的 Provider、Skills、MCP、模型和会话身份保持稳定；本任务只把 Agent lifecycle 安装形态收敛为 desktop。

5. **来源元数据不成为任意下载能力。**
   Claude/OpenCode metadata 只能选择代码内固定 endpoint kind；manifest/API 中的 URL 不进入 renderer 或下载器。

6. **一个可执行软件安装基础设施。**
   source adapter 提供 release descriptor；现有 shared downloader、job、DMG transaction、inventory 和 helper 完成执行。不得复制 Codex 或上游 updater。

7. **不确定性保持不确定。**
   扫描错误、unknown、unavailable 不等于未安装；mirror 可下载不等于产品服务在当地可使用。

## 2. Target architecture

```text
Backend Agent Catalog (canonical identity/order/official links)
  + Frontend PRODUCT_DIRECTORY (local assets + directory priority metadata)
  + Agent readiness scan (authoritative runtime evidence)
        |
        v
AgentDirectoryOrderProjection
  canonical while initial scan is incomplete
  committed stable order after a completed scan
        |
        v
AgentDirectory cards


AgentLifecyclePolicy (single backend owner)
  product + surface -> install/update/launch policy + source kind
        |
        +--> readiness projection
        +--> inventory action eligibility
        +--> source resolution decision
        +--> start_agent_action admission
        |
        v
Fixed product source adapter
        |
        v
Existing artifact/job/DMG/target/helper owners
```

The design adds two narrow owners:

- one pure frontend directory-order projection;
- one crate-private backend lifecycle policy owner.

It does not add a new installer framework.

## 3. Product and surface contract

### 3.1 Stable product IDs

Keep the existing seven IDs:

```text
qoderwork
trae-work
workbuddy
grokbuild
codex
claude-code
opencode
```

Do not introduce `claude-desktop` as an eighth product. That would duplicate all of the configuration, Skills/MCP, models and catalog relationships currently owned by `claude-code`.

### 3.2 Agent lifecycle surfaces

The lifecycle surface matrix becomes:

```text
qoderwork   -> desktop
trae-work   -> desktop
workbuddy   -> desktop
grokbuild   -> cli
codex       -> desktop
claude-code -> desktop
opencode    -> desktop
```

The TypeScript and Rust legal-surface maps remain closed and are updated together. Requests carrying removed `cli` surfaces for Claude/OpenCode fail closed.

### 3.3 Display semantics

- The existing product/catalog label `Claude Code` may remain where it names the configuration target and assignment identity.
- The install component and lifecycle action label must say `Claude Desktop` because the managed artifact is `Claude.app`, not a CLI executable.
- OpenCode shows one desktop lifecycle component; no CLI/desktop pair remains.

This avoids a broad product rename while making the physical software being installed explicit.

## 4. Backend lifecycle policy owner

### 4.1 Shape

Exact module/file placement is chosen after the predecessor task settles, but one private owner must expose semantics equivalent to:

```rust
struct AgentLifecyclePolicy {
    surfaces: &'static [AgentSurface],
    install: bool,
    update: bool,
    launch: bool,
    managed_desktop_source: Option<ManagedDesktopSourceId>,
}

fn lifecycle_policy(
    agent_id: AgentCatalogId,
    surface: AgentSurface,
) -> Result<&'static AgentLifecyclePolicy, AgentReasonCode>;
```

Possible source IDs:

```text
QoderWork
TraeWork
WorkBuddy
ClaudeDesktop
OpenCodeDesktop
CodexDesktopDedicated
GrokCliTooling
```

This is a policy facade, not a universal installer. It contains closed enums and booleans only—no URL, path, command, hash or renderer data.

### 4.2 Required consumers

The same owner drives:

1. Rust `legal_surfaces` / default surface;
2. inventory candidate/destination `installEligible` and `updateEligible` projection;
3. readiness `allowedActions` and `updateState`;
4. whether remote source resolution is needed;
5. `start_agent_action` admission before target lookup/network/side effects;
6. focused contract tests.

The official-link catalog remains a separate semantic owner, but tests must assert that its CLI/desktop link shape agrees with the lifecycle surface contract.

### 4.3 Action admission order

```text
parse closed request
  -> resolve requested/default surface
  -> lifecycle_policy admission
  -> validate action supported
  -> validate opaque target/release binding
  -> refresh inventory/source as required
  -> start job / launch
```

Rejected actions must stop before:

- metadata HTTP;
- artifact download;
- helper authorization;
- DMG mount;
- filesystem mutation;
- application launch.

Use `action_not_supported` for a valid product/surface whose product policy disallows the requested action. Use `surface_not_supported` only for a removed/illegal surface.

## 5. Install-only policy for the domestic products

### 5.1 Readiness flow

Current desktop readiness resolves source before fully deciding the action. The new flow should be evidence- and policy-driven:

```text
inventory = probe product/surface
policy = lifecycle_policy(product, desktop)

if inventory proves not installed and policy.install:
    resolve source for fresh install
elif inventory proves installed and policy.update:
    resolve source for version comparison/update
else:
    do not resolve remote source

project allowedActions from policy AND evidence
```

For QoderWork, TRAE Work and WorkBuddy:

- `policy.install = true`
- `policy.update = false`
- `policy.launch = true`

An installed product therefore needs no network call merely to produce readiness.

### 5.2 Source resolvers remain

The existing resolvers remain because fresh install still needs them:

- QoderWork `latest.yml`/`latest-mac.yml` and fixed DMG aliases;
- TRAE Work CN latest API and exact `data.solo` branch;
- WorkBuddy `/v2/update` release metadata endpoint and DMG rewrite.

The WorkBuddy endpoint name is a vendor API name, not permission to expose a FyAgent update action.

### 5.3 Inventory eligibility

Platform evidence may prove that an installed target is writable and replaceable, but product policy still makes its effective `updateEligible=false`.

Recommended projection:

```text
effective_install_eligible = evidence.install_eligible && policy.install
effective_update_eligible  = evidence.update_eligible  && policy.update
effective_launch_eligible  = evidence.launch_eligible  && policy.launch
```

Do not delete useful path/scope/writability evidence; only apply the action policy at the authoritative projection.

## 6. Directory ordering projection

### 6.1 Metadata owner

Extend the existing `ProductDirectoryEntry` with one closed field, for example:

```ts
type AgentDirectoryPriority = "domestic" | "standard";

type ProductDirectoryEntry = {
  // existing fields
  directoryPriority: AgentDirectoryPriority;
};
```

Only the three user-designated products use `domestic`. Do not maintain another page-local ID array.

### 6.2 Classification

```ts
type AgentDirectoryOrderBucket =
  | "installed_domestic"
  | "installed_other"
  | "unresolved"
  | "not_installed";
```

Classification rules:

| Current scan fact | Bucket |
| --- | --- |
| `installed` or `installed_not_runnable`, domestic | `installed_domestic` |
| `installed` or `installed_not_runnable`, standard | `installed_other` |
| confirmed `not_installed` | `not_installed` |
| `unknown`, `unavailable`, pending, no current result, technical failure | `unresolved` |
| retained old installed result plus current scan failure | `unresolved` for ordering; retain stale card data for display/configuration |

Current scan failure must dominate stale readiness only for ordering classification. It does not rewrite the existing stale-data display contract.

### 6.3 Stable ordering lifecycle

Use a committed-order model:

```text
idle / first scan in progress:
  canonical order

first complete scan:
  calculate + commit bucket order

rescan in progress:
  preserve previous committed order

rescan complete:
  calculate + replace committed order once

authoritative lifecycle reread while not scanning:
  recalculate from the patched readiness
```

This avoids seven card moves during one progressive scan.

### 6.4 Pure helper

The classification/sort itself should be a pure helper colocated with the Agent directory scan projection. It receives:

- current catalog entries;
- scan state/current failures;
- canonical index/product metadata.

It returns a new entry array and never mutates the backend catalog, query cache or scan results.

Sort key:

```text
(bucket_rank, canonical_index)
```

No locale/name sorting is required.

### 6.5 Focus and keys

Cards continue to use `entry.id` as React key. Reordering must not recreate IDs or wrap cards in index-based keys. Tests should keep focus on a card action while the completed order is committed.

## 7. Claude Desktop source adapter

### 7.1 Fixed endpoints

```text
metadata: https://claudeapp.agentsmirror.com/latest/manifest
artifact: https://claudeapp.agentsmirror.com/latest/mac
official fallback page: https://claude.com/download
official upstream redirect identity:
  https://api.anthropic.com/api/desktop/darwin/universal/dmg/latest/redirect
```

No caller supplies these strings.

### 7.2 Metadata parser

Parse a bounded body into a private DTO equivalent to:

```text
schemaVersion == 2
version: bounded dotted version
sources.macos.universal:
  platform == darwin
  arch == universal
  format == dmg
  version == top-level version
  redirect == fixed official upstream redirect identity
  contentLength?: positive bounded hint
```

Fields such as `url`, `fileName`, `buildHash`, `sha256`, `etag`, `lastModified` and `assetName` may exist but are ignored for download authority and executable admission.

The parser should follow the Codex source owner’s existing principles:

- one-mebibyte metadata cap;
- fixed metadata endpoint enum;
- retry/cancellation/cache through existing HTTP infrastructure;
- force refresh before install/update;
- opaque release ID from product/platform/arch/format/version/fixed endpoint kind;
- no remote URL capability.

Prefer extracting a product-neutral fixed-manifest transport primitive from the existing Codex owner only when that preserves its safety ordering. Otherwise delegate through a narrow shared helper; do not copy the full Codex source module.

### 7.3 Artifact and product policy

```text
Product: ClaudeCode
Surface: Desktop
Format: DMG
Architecture: host arm64/x86_64; artifact itself universal
Expected bundle ID: com.anthropic.claudefordesktop
Version source: CFBundleShortVersionString, fallback CFBundleVersion per shared reader
Version equivalence: exact
```

The source adapter hands the fixed artifact endpoint to the existing streamed downloader. The managed DMG transaction then:

1. mounts read-only;
2. finds exactly one direct top-level `.app`;
3. applies Claude’s closed bundle/version policy;
4. stages and commits through existing user/system target owners;
5. rereads inventory/version.

The one-time observed Team ID and notarization status are HIL/provenance evidence. This task must not introduce a new remote publisher/hash admission policy that conflicts with the executable-installer contract.

### 7.4 Service availability disclaimer

The UI may describe the fixed mirror as a download source optimized for installer reachability. It must not state or imply:

- Claude accounts can be created in unsupported regions;
- service/login/model requests will work;
- FyAgent bypasses Anthropic policy;
- the mirror is an official Anthropic service.

## 8. OpenCode Desktop source/update

### 8.1 Separate metadata and artifact authority

Recommended flow:

```text
fixed GitHub latest endpoint for anomalyco/opencode
  -> parse stable tag version only through existing GitHub metadata owner
  -> create release descriptor with host architecture
  -> download through existing fixed stable DMG endpoint
  -> require mounted app version to match frozen descriptor
```

Fixed artifact endpoints remain:

```text
arm64: https://opencode.ai/download/stable/darwin-aarch64-dmg
x64:   https://opencode.ai/download/stable/darwin-x64-dmg
```

The metadata response cannot select another repository or artifact URL.

### 8.2 Reuse boundary

The repository already contains a GitHub latest-version helper for OpenCode CLI. At implementation time:

- reuse it directly if its facade is no longer CLI-specific and preserves bounded/fixed-repository behavior; or
- extract a small crate-private `FixedGithubLatestRelease` owner consumed by Tooling and Agent Desktop.

Do not add another GitHub client stack or call the Tauri command layer from Agent install.

### 8.3 Upstream updater is reference-only

OpenCode Desktop ships its own Electron updater and release metadata. FyAgent must not invoke it because it would bypass:

- FyAgent job/cancellation/progress;
- selected inventory target and exact location;
- `/Applications` helper boundary;
- rollback/recovery state;
- authoritative post-install readback.

Its official releases and asset naming are source evidence only.

## 9. Catalog and official links

Update the static Agent Catalog atomically with contract version/tests:

### Claude

```text
desktop -> https://claude.com/download
```

Remove the CLI official link from Agent Catalog. Product configuration/assignment labels remain unchanged unless a neighboring copy explicitly describes the physical installer.

### OpenCode

```text
product -> https://opencode.ai
desktop -> https://opencode.ai/download
```

Remove the CLI official link.

The frontend strict parser and order/link tests must accept the new exact shape and reject old CLI-bearing fixtures after the contract bump.

## 10. Install/update transaction and helper boundary

No new filesystem transaction is designed here.

```text
DownloadedArtifact
  -> existing managed DMG preparation
  -> existing target authority
  -> existing same-volume transaction
  -> current-user commit OR MacSystemCommitPort
  -> authoritative inventory readback
```

For `/Applications`:

- depend on `08-31-macos-privileged-application-commit-helper`;
- pass only a backend-owned operation capability/FD through that owner;
- do not add sudo, administrator AppleScript, renderer paths or a product-specific helper.

Until the helper is available and signed HIL passes, system targets stay non-actionable with `authorization_required`. This task may still complete source/order/policy code, but must not claim system install/update acceptance.

## 11. Frontend projection

### 11.1 Lifecycle component shape

After surface convergence:

```text
QoderWork CN  Desktop  [一键安装] | 已安装 [打开软件]
TRAE Work CN  Desktop  [一键安装] | 已安装 [打开软件]
WorkBuddy     Desktop  [一键安装] | 已安装 [打开软件]
Claude Code   Claude Desktop [一键安装/一键更新/打开软件]
OpenCode      Desktop  [一键安装/一键更新/打开软件]
```

There is no CLI component row for Claude/OpenCode.

### 11.2 Backend truth

The existing primary-action projection remains generic:

- `not_installed + allowed install` -> install;
- `installed + allowed update` -> update;
- launch remains a separate explicit action.

No product-specific `if (qoderwork)` UI branch should hide update. The backend policy is the truth; frontend tests assert the result.

### 11.3 Progress

Claude/OpenCode use the same lifecycle job transfer projection already being established by the predecessor task:

- one-decimal percentage;
- transferred bytes and rate when observable;
- stable terminal success/failure/cancelled;
- no auto-launch after install/update.

## 12. Error semantics

Expected stable reasons:

| Condition | Reason |
| --- | --- |
| valid product/surface but action disabled by policy | `action_not_supported` |
| removed Claude/OpenCode CLI surface | `surface_not_supported` |
| fixed Claude mirror unavailable or schema invalid | existing `source_not_verified` |
| frozen release changed before action | existing `refresh_required` |
| mounted app version differs from selected release | existing refresh/source mismatch mapping |
| target changed or ambiguous | existing target reason |
| helper unavailable for `/Applications` | `authorization_required` or helper-specific reason owned by helper task |

Do not expose raw HTTP errors, local paths, manifest URLs or mirror internals over renderer IPC.

## 13. Contract/version migration

Expected version changes may include:

- Agent Catalog contract: official link shape changed;
- Agent install readiness/action contract: legal surface matrix and reason enum changed;
- frontend parser fixtures: exact surfaces and reason codes changed.

Perform each bump once at the owning boundary. Do not add compatibility aliases that continue accepting removed CLI surfaces indefinitely.

## 14. Testing strategy

### 14.1 Pure/order tests

- every bucket combination;
- stable canonical ties;
- installed-not-runnable;
- stale installed + current failure -> unresolved order but configurable card;
- initial scan and rescan freeze;
- post-action reorder;
- input array not mutated;
- focus/card key preserved.

### 14.2 Backend policy tests

- complete product/surface/action matrix;
- Chinese update rejected before transport/target/side effect;
- installed Chinese readiness makes zero source calls;
- not-installed Chinese resolves source and installs;
- Claude/OpenCode CLI surface rejected;
- Claude/OpenCode desktop install/update/launch admitted only with valid evidence.

### 14.3 Source tests

Claude:

- exact schema v2 universal branch;
- top-level/branch version mismatch;
- wrong platform/arch/format/redirect;
- oversized body;
- extra URL/hash fields ignored as authority;
- fixed endpoint selection;
- cache/force refresh/retry/cancel;
- real metadata fixture and mounted-app fixture.

OpenCode:

- fixed repository/tag parsing;
- prerelease/draft/invalid tag rejection as applicable to the reused owner;
- arm64/x64 stable endpoint mapping;
- metadata/download race -> mounted version mismatch/refresh;
- no upstream updater invocation.

### 14.4 Integration/HIL

- Claude fresh install/update/launch from current-user scope;
- OpenCode fresh install/update/launch;
- existing `/Applications` target via signed helper build;
- rollback after post-commit verification failure;
- app-running refusal;
- network throttling/progress;
- Chinese three install-only behavior;
- directory order after scan and after install.

## 15. Rollout and rollback

Implement in reversible slices:

1. backend product policy and tests;
2. surface/catalog convergence;
3. pure directory order projection;
4. Claude source/product policy;
5. OpenCode metadata/update refinement;
6. frontend integration;
7. helper-backed system HIL and specs.

If Claude source or system helper acceptance fails:

- keep Claude action disabled with a precise reason and official-page fallback;
- do not re-enable CLI install as an implicit fallback;
- do not weaken the fixed-source or helper boundary;
- retain the independently valid sorting and install-only policy changes.
