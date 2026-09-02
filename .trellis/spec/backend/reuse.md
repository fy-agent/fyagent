# Backend Reuse Contract

## 1. Scope / Trigger

Read this contract before adding a Rust/Tauri service, helper, parser, protocol
client, filesystem/archive primitive, platform utility, or Cargo dependency.
It also applies when implementation reveals code that another backend domain is
likely to consume.

Backend work follows this decision order:

1. reuse or minimally extend the existing FyAgent owner;
2. reuse an already-adopted crate or Rust/std/Tauri primitive;
3. when no suitable current capability exists, research maintained open-source
   crates before implementing a bespoke replacement;
4. place FyAgent-specific semantics behind one crate-scoped shared owner or
   stable facade;
5. write a bespoke implementation only when the earlier options are unsuitable
   and record the concrete reason for non-trivial cases.

This contract does not authorize widening Rust visibility, adding dependencies
without review, or bypassing the ownership/security rules in
[Rust Host Modular Boundaries](./modular-boundaries.md).

## 2. Signatures

Reusable backend owners stay internal by default. Callers depend on one
explicit facade rather than importing implementation modules:

```rust
pub(crate) mod reusable_domain;
pub use reusable_domain::ReusableService;

// reusable_domain/mod.rs
mod parser;
mod platform;

pub struct ReusableService {
    // owned state
}
```

Pure shared helpers that genuinely have several backend consumers remain
crate-scoped unless an external contract requires otherwise:

```rust
pub(crate) fn normalize_input(input: &str) -> Result<NormalizedInput, Error>;
```

Transport stays thin and delegates to the shared owner:

```rust
#[tauri::command]
pub async fn command_name(
    state: State<'_, AppState>,
    request: WireRequest,
) -> Result<WireResponse, String> {
    state.reusable_service.execute(request).await.map_err(|err| err.to_string())
}
```

Adding a crate is a dependency decision, not a shortcut around ownership. The
manifest/lockfile remain the dependency signatures:

```text
src-tauri/Cargo.toml
src-tauri/Cargo.lock
```

## 3. Contracts

### Search existing owners first

Before writing a new backend capability:

1. search the owning service and sibling services for the same behavior;
2. search `src-tauri/Cargo.toml` for an adopted crate that already owns the
   primitive;
3. prefer standard-library/Tauri/current-crate APIs over a new local copy;
4. extend the current owner when the semantic responsibility is the same.

FyAgent already has established owners for HTTP, URL handling, TOML/JSON
serialization, archive operations, hashing/cryptography, async runtime,
database access, platform/runtime orchestration, and Skill filesystem safety.
Do not create parallel protocol, archive, path-safety, or transaction logic
merely because a new feature is implemented in another service.

macOS privileged helper Bless / Authorization / XPC wrapping reuses the
pinned Swift packages Blessed `0.6.0`, Authorized `1.0.0`, and SecureXPC at
revision `1cece54562c7626d042f007d2f38cfe325565850`. Do not rewrite
`SMJobBless`, invent a generic XPC RPC, or add a second helper per product.
See [macOS Privileged System-Commit Helper](./macos-system-commit.md).

### Research open-source candidates before bespoke code

If no current owner or adopted crate solves a non-trivial capability, research
maintained open-source candidates from primary sources before writing the
replacement locally. Review at least:

- required API/capability in the intended version;
- license compatibility;
- maintenance/release activity and credible ownership;
- security/provenance and known advisory exposure;
- supported Rust/toolchain version and FyAgent's macOS/Windows production plus
  Linux development-host needs;
- default/features/transitive dependencies, binary size/build-time impact, and
  duplication with existing crates;
- async/runtime/TLS/serialization choices so a new crate does not introduce a
  competing stack without necessity;
- whether one small FyAgent adapter can isolate upstream API churn.

Prefer an already-adopted crate over a new dependency when both meet the need.
Prefer a focused crate over a broad framework when only one primitive is
missing. Security-sensitive primitives such as cryptography, URL parsing,
archive parsing, HTTP/TLS, and structured serialization should not be
reimplemented locally when a reviewed existing owner/crate already provides
the required behavior.

### Promote reusable discoveries early

If implementation reveals a capability with a concrete second consumer, move
or propose it under one shared owner before merge rather than allowing a second
copy to form.

- Shared project capability does **not** imply broad `pub` visibility.
- Keep implementation modules private; expose the minimum crate-scoped helper
  or stable facade callers require.
- If two domains merely share a few coincidentally equal literals, do not
  manufacture a shared abstraction. The shared concept must have one semantic
  reason to change together.
- Cross-domain adapters call the owner; they do not clone its validation,
  filesystem, retry, rollback, or security rules.

### Bespoke implementation exception

A local implementation is acceptable when reuse candidates are unsuitable,
for example when the behavior is truly tiny and stable, dependency cost would
be disproportionate, platform support is wrong, licensing is incompatible, or
the external API would violate an established FyAgent boundary. For non-trivial
cases, record the evaluated alternatives and rejection reason in the task,
design, research, or review artifact.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| A service duplicates an existing parser/archive/path/security/transaction owner | Reject; delegate to the established owner |
| A new crate is added without checking current `src-tauri/Cargo.toml` capabilities | Reject; perform the adopted-dependency search first |
| No current capability exists and non-trivial bespoke code is added without open-source candidate review | Reject or record why external reuse is inapplicable before merge |
| A new crate duplicates the existing HTTP/TLS/serialization/runtime stack without a concrete gap | Reject; reuse the current stack |
| A dependency has incompatible license/platform/toolchain/security characteristics | Reject that candidate |
| A second real backend consumer appears but the first implementation remains domain-local | Promote/propose one crate-scoped owner before merge |
| Shared code is made broadly `pub` only to enable reuse inside the crate | Reject; keep modules `pub(crate)`/private and re-export the minimum facade |
| A one-off helper is generalized with speculative parameters and no second consumer | Keep it local; avoid premature abstraction |
| Existing safety/rollback/validation ordering would be weakened by extraction | Keep the cohesive owner; reuse through delegation rather than splitting the invariant |

## 5. Good / Base / Bad Cases

- **Good:** a new Skill path reuses the existing archive/materialization safety
  owner instead of implementing ZIP traversal/resource limits again.
- **Good:** a missing capability is first checked against current crates, then
  compared with maintained open-source candidates; the chosen crate is wrapped
  behind one crate-scoped FyAgent adapter.
- **Good:** two services need the same stable pure normalization rule, so one
  `pub(crate)` helper becomes the semantic owner with focused tests.
- **Base:** a one-off five-line transformation stays local because there is no
  concrete second consumer and a dependency/shared facade would add more
  surface than it removes.
- **Base:** a large existing service remains cohesive when mutation/rollback or
  security ordering is itself the invariant; callers reuse it by delegation.
- **Bad:** a new feature copies URL parsing, HTTP retries, archive extraction,
  path confinement, or hashing because the existing owner lives in another
  module.
- **Bad:** add a crate merely because it saves a few lines, without checking
  license, maintenance, advisories, platform/toolchain support, features, or
  transitive cost.
- **Bad:** make internal modules public across the crate/repository to label
  them "reusable" instead of designing a narrow shared facade.

## 6. Tests Required

For a SPEC-only update to this contract, run the repository Markdown/spec
checks applicable to `.trellis/spec/**` changes.

For backend implementation or dependency changes governed by this contract,
run at least:

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

Also run the focused architecture/security tests owned by the affected service.
When ownership or visibility changes, include:

```bash
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
```

When a new crate is introduced, review the locked dependency graph and ensure
the manifest/lockfile change contains only the intended dependency/features.
Hosted dependency/security checks remain authoritative for advisories and
repository policy; this SPEC does not waive them.

## 7. Wrong vs Correct

Wrong: duplicate an established capability in the new feature.

```rust
fn extract_new_feature_archive(bytes: &[u8]) -> Result<PathBuf, String> {
    // New feature-local ZIP/path-safety implementation.
    todo!()
}
```

Correct: call the existing owner and keep one safety contract.

```rust
let installed = skill_service.install_validated_archive(request).await?;
```

Wrong: widen visibility because another backend module needs one operation.

```rust
pub mod internal_parser;
```

Correct: keep internals private and expose one narrow project-owned facade.

```rust
mod internal_parser;

pub(crate) fn parse_supported_input(input: &str) -> Result<ParsedInput, Error> {
    internal_parser::parse(input)
}
```

Wrong: start a non-trivial local implementation without first checking current
owners/adopted crates and viable open-source candidates.

Correct: record the reuse search, adopt/extend the best fitting owner, and use a
bespoke implementation only when the documented alternatives fail the project
constraints.
