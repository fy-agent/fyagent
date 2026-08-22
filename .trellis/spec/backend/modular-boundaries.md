# Rust Host Modular Boundaries

## 1. Scope / Trigger

Read this before adding a Tauri command, growing a backend service, moving code
between `commands`, `services`, Provider/Proxy/config owners, or creating a new
Rust crate. FyAgent is a modular monolith: prefer private Rust submodules plus
an explicit facade before considering a Cargo package split.

## 2. Signatures

Transport functions keep stable wire signatures while delegating:

```rust
#[tauri::command]
pub async fn command_name(/* wire DTO/state */) -> Result<WireResult, String> {
    // translate / validate / delegate
}
```

Service implementations are crate-scoped and selected facades are re-exported:

```rust
pub(crate) mod provider;
pub(crate) mod proxy;
pub(crate) mod skill;

pub use provider::ProviderService;
pub use proxy::ProxyService;
pub use skill::SkillService;
```

Private implementation subdomains include `provider/universal.rs`,
`provider/common_config.rs`, `skill/assignment.rs`, `skill/discovery.rs`,
`skill/marketplace.rs`, `skill/migration.rs`, `skill/repository.rs`,
`proxy/takeover.rs`, `codex_config/auth.rs`, `codex_config/catalog.rs`,
`codex_config/features.rs`, and `codex_config/storage.rs`.
Tooling business policy lives behind crate-scoped `services/tooling.rs`; its
private `versions`, `lifecycle`, `discovery`, and `terminal` modules own the
corresponding application/domain responsibilities while the Tauri command
module remains only the transport facade.

## 3. Contracts

- `commands/**` owns Tauri transport; domain behavior belongs in its service or
  config owner. Do not recreate `commands/misc.rs`.
- Lifecycle/system utility commands belong in `commands/system.rs`; CLI/tool
  install/probe/terminal command **wrappers** belong in `commands/tooling.rs`.
  `commands/tooling.rs` must stay limited to the four reviewed wire commands
  (`get_tool_versions`, `run_tool_lifecycle_action`,
  `probe_tool_installations`, `open_provider_terminal`). Version probing,
  installation discovery, lifecycle command planning/execution, Windows
  fail-closed policy and terminal launch behavior belong to the Tooling service
  owner. Within that owner:
  - `tooling/versions.rs` owns local/remote version projection, npm/GitHub/PyPI
    latest-version policy and semver/pre-release selection;
  - `tooling/lifecycle.rs` owns install/update allowlists and command policy;
  - `tooling/discovery.rs` owns installation-distribution reports, conflict/
    confirmation projection and the constrained detected-tool execution entry;
  - `tooling/terminal.rs` owns provider-terminal orchestration, environment
    projection, launch-directory validation and interactive terminal command
    launch.
  Cross-capability shell/path/platform primitives may remain in the parent
  service when they are genuinely shared. Do not duplicate them only to make
  every child module self-contained. Backend siblings call the service owner,
  never command internals.
- `services/mod.rs` modules are `pub(crate)`; stable caller APIs are explicit
  re-exports.
- Provider/Skill/Proxy/Codex implementation subdomains are private `mod` and
  the parent remains the compatibility facade.
- `codex_config/storage.rs` owns path/file/atomic-write behavior.
  `write_codex_live_atomic` validates TOML before mutation and restores old
  `auth.json` bytes if the config write fails.
- `provider/universal.rs` preserves unknown nested settings unless projection
  overrides the same field.
- `provider/common_config.rs` owns the **pure** common-config policy: sensitive
  credential-key classification and per-application extraction for Claude,
  Codex, Gemini, OpenCode and OpenClaw. Database writes, mutation guards and
  the ordered Gemini credential-scrub transaction remain in `ProviderService`;
  that order is a safety property and must not be fragmented for file size.
- `skill/discovery.rs` owns discovery filtering/pagination/cache; ZIP, symlink,
  copy, backup, and vendor safety stay outside it.
- `skill/marketplace.rs` owns skills.sh / SkillHub HTTP DTOs, slug/category/URL
  validation and response mapping. Marketplace install delegates back to the
  existing bounded download/archive installation primitives; do not create a
  second ZIP extraction or archive-budget implementation in marketplace code.
- `skill/repository.rs` owns `.agents` lock parsing, repository coordinate /
  branch derivation, repository metadata persistence and repo-list CRUD.
- `skill/migration.rs` owns the first-start SSOT migration application flow;
  it reuses the parent service's validated copy/hash/path primitives rather
  than creating a second filesystem-safety implementation.
- `skill/assignment.rs` owns target enable/disable and database-to-target
  synchronization orchestration. It delegates materialization/removal to the
  existing safety primitives. Archive extraction, symlink/traversal/resource
  budgets, vendor copy, backup-before-delete and materialization ordering stay
  under one cohesive Skill filesystem/transaction owner.
- `proxy/takeover.rs` owns pure takeover URL/config matching; state transitions
  and live I/O stay in `ProxyService` unless separately justified.
- `codex_config/auth.rs` owns Codex login-material classification, OAuth/API-key
  policy, stale third-party auth residue detection/cleanup and token-backfill
  policy. `codex_config` remains the stable facade and re-exports only the
  functions current callers actually require; test-only/private predicates do
  not become public merely because they moved files.
- `codex_config/features.rs` owns native capability state, diagnostics, draft
  patching, save validation/defaulting and non-sensitive warning projection.
- `codex_config/catalog.rs` owns the model-catalog domain end-to-end: tool
  profile selection, provider model-spec projection, template/cache/CLI
  fallback, vendor-official catalog projection, parser-required backfill,
  `model_catalog_json` / owned `web_search` projection, bounded catalog
  readback and path/symlink confinement. The parent `codex_config` module stays
  the stable facade and owns live/provider/proxy/session/MCP transaction
  coordination; do not move those ordered mutations into the catalog module.
- `proxy/handlers.rs`, `RequestForwarder`, Provider mutation/rollback
  coordination, and Skill archive/materialization safety may intentionally
  remain physically large. Their streaming/failover/rollback/filesystem order
  is stronger evidence than line count; split them only when a one-way pure or
  independently testable seam is proven.
- Module moves preserve serialized DTOs, registration, persistent paths,
  validation order, security checks, and error semantics.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Bare `pub mod` under `services/mod.rs` | Architecture test fails; use `pub(crate)` and explicit re-export |
| `commands/misc.rs` reintroduced | Architecture test fails; choose an owning command module |
| Tooling implementation markers return to `commands/tooling.rs` | Architecture test fails; keep four thin wrappers and move policy to `services/tooling.rs` |
| Tooling version/lifecycle/discovery/terminal orchestration regrows in the parent service | Architecture test fails for reviewed markers; move the responsibility back to its private owner while keeping genuinely shared primitives centralized |
| Provider/Skill/Proxy/Codex subdomain made public | Architecture test fails unless a reviewed external contract requires it |
| Marketplace code starts implementing its own ZIP/archive safety path | Reject; delegate to the existing Skill archive/install owner |
| Skill assignment/migration module starts duplicating archive/vendor/symlink safety primitives | Reject; those modules orchestrate through the single filesystem-safety owner |
| Pure Provider common-config module starts owning DB/locks/scrub sequencing | Reject; transaction/rollback order remains in `ProviderService` |
| Codex catalog module starts owning provider/live/proxy/session transaction ordering | Reject; catalog owns catalog policy/I/O only and delegates live coordination through the parent facade |
| Codex config write fails after auth write | Restore previous auth bytes or delete newly created auth; return error |
| Universal projection sees unknown nested settings | Preserve them while applying overrides |
| Discovery cache mutex is poisoned | Recover inner value; do not panic |
| Invalid discovery status is parsed | Return error; never silently widen to `all` |
| Placeholder cleanup sees HTTPS/non-loopback URL | Do not classify it as local proxy URL |

## 5. Good / Base / Bad Cases

- **Good:** extract one cohesive rule set with tests into a private submodule,
  retain the parent service facade, then tighten visibility.
- **Good:** keep Windows user-scope/fail-closed contracts intact while moving
  command ownership.
- **Good:** keep a stable facade while private modules own Tooling policy,
  Skill assignment/repository/migration, marketplace transport, Codex auth /
  features / catalog policy, or Provider common-config rules.
- **Base:** a large file may remain large if the remaining code is a tightly
  coupled state machine whose sequence is itself a safety property.
- **Base:** `proxy/handlers.rs`, `RequestForwarder`, Skill archive safety or a
  Provider mutation coordinator may remain large after audit when extracting
  them would create peer owners for one protocol/transaction.
- **Bad:** create many crates without reducing coupling or public surface.
- **Bad:** weaken a platform scanner or sealed-structure test merely to make a
  sensitive file move pass.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run test:unit -- tests/remainingPlatformSurface.test.ts
```

Focused iteration should also cover Codex atomic write/rollback, Provider
live/takeover/common-config, all `services::skill::tests`, Proxy
takeover/OAuth restore, Tooling lifecycle/platform boundaries, Windows
user-scope, and desktop security contracts.

## 7. Wrong vs Correct

Wrong:

```rust
pub mod giant_feature;
crate::services::giant_feature::internal::helper();
```

Correct:

```rust
pub(crate) mod giant_feature;
pub use giant_feature::FeatureService;

// giant_feature/mod.rs
mod internal;
```

Do not create a crate because a file crossed an arbitrary line-count limit.
First prove a private module/API boundary; promote it only when package-level
isolation has a concrete build, dependency, or reuse benefit.
