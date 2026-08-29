# Design — Agent 生命周期可靠性治理总架构

## 1. Current Shape

当前 `agent_install` 已有闭集 `agentId + action + expectedReleaseId` façade，但它没有表达“管理哪一份安装”的能力；macOS 部署固定写入用户 Applications，Windows EXE 执行明确 fail closed，Auth 启动后立即返回 success。V2 则把六个页面静态导入并持续挂载，选中态依赖异步浮动 Lens，部分基础交互仍由项目自研。

这些缺口属于不同责任域，不能通过继续扩大 `desktop.rs`、`mod.rs`、`SelectionLens.tsx` 或页面文件来修补。

## 2. Target Ownership Map

```text
Agent Catalog (product/capability policy SSOT)
  |
  +-- Installation Inventory
  |     +-- candidate domain and opaque identity
  |     +-- macOS bundle adapter
  |     +-- Windows registry/App Paths/package/known-path adapters
  |     `-- deduplication, ambiguity and selected-target policy
  |
  +-- Managed Package Coordinator
  |     +-- existing source resolvers
  |     +-- existing bounded download/temp/job primitives
  |     +-- closed package-format deployment adapters
  |     `-- authoritative post-install verification
  |
  +-- Auth Session Coordinator
        +-- Claude adapter
        +-- Grok adapter
        +-- OpenCode provider adapter
        +-- desktop-app launch adapter
        `-- Auth Center/Codex delegation

FeaturePorts
  |
  +-- shared installation target UI
  +-- shared authoritative action status UI
  +-- shared Tabs/selection primitives
  `-- route-owned business panels and drafts
```

Names are descriptive, not a mandate to add top-level crates. Follow the modular-monolith rule: prefer private Rust submodules and one explicit facade. If the existing Codex Desktop owner can be generalized without weakening its invariants, move common candidate/package concepts there or into a private sibling owner rather than duplicating them.

## 3. Shared Backend Owners

### Installation Inventory

One crate-scoped owner aggregates platform evidence into trusted candidates. It owns:

- opaque `candidateId` generation and revision binding;
- scope/owner/package-kind normalization;
- evidence provenance and confidence;
- duplicate collapse without discarding conflicts;
- ambiguity and persisted/explicit selection policy;
- safe renderer projection that does not accept a path back from IPC.

Product adapters contribute evidence; they do not select the winner.

### Managed Package Coordinator

Reuse Codex Desktop capabilities already present in `codex_desktop`: trusted installation candidates, retained downloaded-artifact capability, temp ownership, job/cancellation transitions, platform planning and post-install observation. The common coordinator owns sequencing; product source descriptors and package-format adapters remain closed.

### Post-install Verifier

Every package path must call one shared verification contract after deployment. Verification compares the selected candidate/expected identity/release against a fresh inventory. Product-specific version equivalence remains in product policy, but “copy returned 0” is never success.

### Auth Session Coordinator

Authentication uses a separate state model because user interaction and verification differ from package installation. The coordinator owns session IDs, per-agent single-flight, cancellation, deadlines, stage transitions and redacted diagnostics. Adapters expose only closed operations and verified observations.

## 4. Shared Frontend Owners

### Installation Target UI

Add or extend one shared candidate picker/dialog after Stage 1 freezes the DTO. It receives opaque candidates and emits `candidateId + candidateRevision`; it never accepts arbitrary paths. It is intended for Agent directory install/update and future lifecycle management surfaces.

### Authoritative Action Status

Install and Auth may share visual vocabulary, but not necessarily one domain hook. A shared status surface may render stage, progress, cancellation and reread result. The state machines remain domain-owned. Promote a shared controller hook only after two operations have the same cancellation/readback semantics, not merely similar buttons.

### Tabs and Selection

Keep `FeatureTabs` as the FyAgent API and migrate its internals to Radix Tabs. Introduce a CSS-first selected treatment reusable by navigation, tabs and catalog items only if their semantics can remain explicit. `SelectionLens` becomes optional decoration and observes only the active host/track required for animation.

### Route State and Loading

Route modules load lazily. Draft persistence is explicit per business domain: local state for disposable UI, query cache for backend resources, and a small draft owner only where leaving a route must preserve unsaved work. Blanket “visited means mounted forever” is removed as the default.

## 5. Wire Boundaries

Renderer requests remain closed. A future lifecycle request may carry:

```text
agentId
action
candidateId?
expectedCandidateRevision?
expectedReleaseId?
```

It must not carry URL, filesystem path, command, package arguments, installer switches, credential data or bypass flags. Candidate display location is backend-projected and privacy-reviewed; selection always returns the opaque ID.

Auth gets a distinct session DTO and stage enum. Do not overload `AgentActionJobStage` with `awaiting_user` semantics unless a reviewed generalized interactive-action domain proves that both install and Auth share all transition invariants.

## 6. Dependency and Rollout

1. Freeze candidate domain and migration/compatibility in Stage 1.
2. Implement macOS and Windows adapters against that domain.
3. Implement Auth session contract independently.
4. Land frontend reliability work in small slices; connect candidate/Auth UI after their DTOs stabilize.
5. Run final cross-platform integration and installed-app UAT.

Each stage must preserve existing command compatibility or provide an explicit contract version bump. Old callers must fail closed rather than selecting a target implicitly.

## 7. Rollback Strategy

- Contract changes land before destructive deployment changes.
- macOS/Windows execution remains disabled for a product/format until its inventory and verifier are ready.
- New Auth session UI may coexist behind the existing allowed-action projection until each adapter is verified; the legacy immediate-success path must not remain reachable for migrated adapters.
- Frontend refactors should be independently revertible: static selected fallback first, Tabs migration second, route loading/state changes third.
- Any architecture extraction that weakens Codex installer safety, Windows user-context isolation or renderer input closure is rejected, even if it reduces duplication.

## 8. Architecture Review Gate

Before each child enters implementation, reviewers must answer:

1. Which current owner is being reused or extended?
2. What exact semantic responsibility receives one new owner?
3. Which second consumer justifies a shared module/component?
4. Does the change enlarge IPC, filesystem, shell or credential authority?
5. Can failure preserve the currently usable installation/configuration?
6. What authoritative readback proves success?

An answer based only on file size, visual similarity or future hypothetical reuse is insufficient.
