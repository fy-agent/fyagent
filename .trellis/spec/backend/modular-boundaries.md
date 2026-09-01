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
Agent lifecycle authority lives behind crate-scoped `agent_install`; its
private `inventory`, `desktop`, `windows`, `cli`, `auth`, `source`, `fetch`,
`jobs`, and `lifecycle_policy` modules own domain policy while
`commands/agent_install_readiness.rs` remains transport-only.
macOS `/Applications` last-write authority lives behind crate-scoped
`macos_system_commit` (`MacSystemCommitPort`, product/slot policy, C ABI).
That module is not a Tauri command owner and must not grow a renderer
path/URL/command surface. See
[macOS Privileged System-Commit Helper](./macos-system-commit.md).

Target-exclusive parent imports and private-owner tests must carry the same
target boundary as their production consumer:

```rust
#[cfg(target_os = "macos")]
use lifecycle::npm_install_command_for;

// services/tooling/lifecycle.rs
#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
}
```

## 3. Contracts

- `commands/**` owns Tauri transport; domain behavior belongs in its service or
  config owner. Do not recreate `commands/misc.rs`.
- `agent_install/inventory.rs` is the single owner of installation candidate
  normalization, provenance merge, opaque snapshot/target/revision
  capabilities, expiry, stale revalidation, and implicit-selection policy.
  `desktop.rs`, Tooling/CLI adapters, Codex Desktop, and later registry/MSIX
  adapters emit evidence or expose a narrow trusted-status adapter; they do not
  choose a winner, mint renderer IDs, or implement separate dedup/revision
  algorithms. The command layer must never expose backend paths/registry
  identities or accept a renderer path. Product execution modules receive a
  validated backend capability only after inventory re-enumeration.
- `agent_install/lifecycle_policy.rs` is the single owner of legal surfaces,
  default surface, and whether `install` / `update` / `launch` is admitted for
  a catalog product. Readiness, inventory, and `start_agent_action` consult it
  before source fetch or file mutation. Do not keep a second product-action
  matrix in the renderer, catalog copy, or a page-local `Set`.
- `macos_system_commit` owns the frozen helper product/slot table, C ABI
  request, and production-disabled `MacSystemCommitPort`. `agent_install` and
  Codex call `system_scope_rejection()` for system targets while
  `production_enabled()` is false. Do not open XPC from the command layer or
  from the renderer.
- `agent_install/windows.rs` owns Windows desktop evidence normalization,
  registry/App Paths adapters, Win32 version/file/signature inspection, and
  the three closed Agent EXE product policies. It does not own a downloader,
  ProgramData bridge, helper executable, renderer command, or arbitrary
  ShellExecute surface. Those reusable executable-installer primitives remain
  in the existing Codex Desktop/`fyagent-user-helper` owner and are exposed to
  Agent install only through crate-private closed action/product APIs. Codex
  PackageManager policy and the Agent job slot remain separate consumers of
  those primitives.
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
    Parent `use` declarations must be gated by the targets where parent
    production code actually consumes the symbol. A test-only child consumer or
    a broad `#[cfg(test)] use lifecycle::*` does not justify an unconditional
    parent import. Windows-only lifecycle policy tests stay in
    `tooling/lifecycle.rs` under `#[cfg(all(test, target_os = "windows"))]` so
    private helpers remain private; do not hoist those tests into `tooling.rs` or
    widen helper visibility merely to make another target compile.
    Cross-capability shell/path/platform primitives may remain in the parent
    service when they are genuinely shared. Do not duplicate them only to make
    every child module self-contained. Backend siblings call the service owner,
    never command internals.
- `services/mod.rs` modules are `pub(crate)`; stable caller APIs are explicit
  re-exports.
- `services/secret` is a crate-scoped native security owner. Its standalone
  core remains unregistered while integration tests/native HIL compile it;
  add the `services/mod.rs` registration with the first production consumer
  instead of suppressing dead-code warnings for a dormant module.
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

| Condition                                                                                          | Required result                                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bare `pub mod` under `services/mod.rs`                                                             | Architecture test fails; use `pub(crate)` and explicit re-export                                                                                                                       |
| `commands/misc.rs` reintroduced                                                                    | Architecture test fails; choose an owning command module                                                                                                                               |
| Tooling implementation markers return to `commands/tooling.rs`                                     | Architecture test fails; keep four thin wrappers and move policy to `services/tooling.rs`                                                                                              |
| Tooling version/lifecycle/discovery/terminal orchestration regrows in the parent service           | Architecture test fails for reviewed markers; move the responsibility back to its private owner while keeping genuinely shared primitives centralized                                  |
| Parent imports a target-exclusive Tooling child API without the matching `cfg`                     | Matching-target `cargo clippy --all-targets -- -D warnings` fails on an unused import or a target-only symbol leaks into the wrong build; gate the import with the production consumer |
| Platform-only lifecycle tests are hoisted to `tooling.rs` or a private helper is widened for tests | Reject; colocate the tests in `tooling/lifecycle.rs` with `#[cfg(all(test, target_os = "<target>"))]`                                                                                  |
| Provider/Skill/Proxy/Codex subdomain made public                                                   | Architecture test fails unless a reviewed external contract requires it                                                                                                                |
| Marketplace code starts implementing its own ZIP/archive safety path                               | Reject; delegate to the existing Skill archive/install owner                                                                                                                           |
| Skill assignment/migration module starts duplicating archive/vendor/symlink safety primitives      | Reject; those modules orchestrate through the single filesystem-safety owner                                                                                                           |
| Pure Provider common-config module starts owning DB/locks/scrub sequencing                         | Reject; transaction/rollback order remains in `ProviderService`                                                                                                                        |
| Codex catalog module starts owning provider/live/proxy/session transaction ordering                | Reject; catalog owns catalog policy/I/O only and delegates live coordination through the parent facade                                                                                 |
| `macos_system_commit` grows a Tauri command or accepts a renderer path/URL                         | Reject; keep crate-private port + closed Agent/Codex actions                                                                                                                           |
| A second legal-surface or install/update matrix appears outside `lifecycle_policy.rs`              | Reject; one product/surface/action owner                                                                                                                                               |
| Codex config write fails after auth write                                                          | Restore previous auth bytes or delete newly created auth; return error                                                                                                                 |
| Universal projection sees unknown nested settings                                                  | Preserve them while applying overrides                                                                                                                                                 |
| Discovery cache mutex is poisoned                                                                  | Recover inner value; do not panic                                                                                                                                                      |
| Invalid discovery status is parsed                                                                 | Return error; never silently widen to `all`                                                                                                                                            |
| Placeholder cleanup sees HTTPS/non-loopback URL                                                    | Do not classify it as local proxy URL                                                                                                                                                  |

## 5. Good / Base / Bad Cases

- **Good:** extract one cohesive rule set with tests into a private submodule,
  retain the parent service facade, then tighten visibility.
- **Good:** keep Windows user-scope/fail-closed contracts intact while moving
  command ownership.
- **Good:** keep a stable facade while private modules own Tooling policy,
  Skill assignment/repository/migration, marketplace transport, Codex auth /
  features / catalog policy, or Provider common-config rules.
- **Good:** keep Windows-only lifecycle helper tests beside the private helper
  and gate a macOS-only parent import with `#[cfg(target_os = "macos")]`.
- **Base:** a large file may remain large if the remaining code is a tightly
  coupled state machine whose sequence is itself a safety property.
- **Base:** a child function may compile on both platforms while only one
  platform's parent production path imports it; the parent import still follows
  the narrower consumer boundary.
- **Base:** `proxy/handlers.rs`, `RequestForwarder`, Skill archive safety or a
  Provider mutation coordinator may remain large after audit when extracting
  them would create peer owners for one protocol/transaction.
- **Bad:** create many crates without reducing coupling or public surface.
- **Bad:** weaken a platform scanner or sealed-structure test merely to make a
  sensitive file move pass.
- **Bad:** leave a child API imported unconditionally because tests use a glob,
  or move target-only tests to the parent to avoid a private-module test block.

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

For Tooling target-gating changes, the architecture test must assert that
platform-private policy remains in `tooling/lifecycle.rs` and that the parent
imports `npm_install_command_for` only inside its macOS-gated import. Local
host-native checks do not prove the opposite target. The pushed SHA therefore
also requires the matching `Backend Checks (Windows)` job, whose all-target
check, Clippy with warnings denied, and Rust tests catch target-only import and
test-compilation regressions.

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

For a target-exclusive child API, this is also wrong:

```rust
// Imported in every production build even though only macOS parent code uses it.
use lifecycle::npm_install_command_for;

// Hoisted only to reach lifecycle-private Windows helpers.
#[cfg(all(test, target_os = "windows"))]
mod windows_lifecycle_tests;
```

Keep the import and tests at their actual ownership boundaries:

```rust
#[cfg(target_os = "macos")]
use lifecycle::npm_install_command_for;

// services/tooling/lifecycle.rs
#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
}
```
