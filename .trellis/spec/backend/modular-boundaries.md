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
`skill/discovery.rs`, `proxy/takeover.rs`, and `codex_config/storage.rs`.

## 3. Contracts

- `commands/**` owns Tauri transport; domain behavior belongs in its service or
  config owner. Do not recreate `commands/misc.rs`.
- Lifecycle/system utility commands belong in `commands/system.rs`; CLI/tool
  install/probe/terminal commands belong in `commands/tooling.rs`.
- `services/mod.rs` modules are `pub(crate)`; stable caller APIs are explicit
  re-exports.
- Provider/Skill/Proxy/Codex implementation subdomains are private `mod` and
  the parent remains the compatibility facade.
- `codex_config/storage.rs` owns path/file/atomic-write behavior.
  `write_codex_live_atomic` validates TOML before mutation and restores old
  `auth.json` bytes if the config write fails.
- `provider/universal.rs` preserves unknown nested settings unless projection
  overrides the same field.
- `skill/discovery.rs` owns discovery filtering/pagination/cache; ZIP, symlink,
  copy, backup, and vendor safety stay outside it.
- `proxy/takeover.rs` owns pure takeover URL/config matching; state transitions
  and live I/O stay in `ProxyService` unless separately justified.
- Module moves preserve serialized DTOs, registration, persistent paths,
  validation order, security checks, and error semantics.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Bare `pub mod` under `services/mod.rs` | Architecture test fails; use `pub(crate)` and explicit re-export |
| `commands/misc.rs` reintroduced | Architecture test fails; choose an owning command module |
| Provider/Skill/Proxy/Codex subdomain made public | Architecture test fails unless a reviewed external contract requires it |
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
- **Base:** a large file may remain large if the remaining code is a tightly
  coupled state machine whose sequence is itself a safety property.
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
live/takeover, all `services::skill::tests`, Proxy takeover/OAuth restore,
Windows user-scope, and desktop security contracts.

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
