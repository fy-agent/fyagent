# Rust and Tauri Modular Boundary Contract

## 1. Scope / Trigger

Read this contract before adding a Rust module, changing `pub`/`pub(crate)`
visibility, registering a Tauri command, moving SQL or filesystem/network/
process logic, exposing a Proxy/Agent submodule, or introducing a second
business implementation path.

The authoritative structure is the current `src-tauri/src/**` module tree plus
`tests/architecture/rustModuleBoundaries.test.ts`. This file describes the
rules; it does not override a tested allowlist with a blanket style slogan.

Related owners:

- [Frontend Modular Boundaries](../frontend/modular-boundaries.md)
- [Reuse](./reuse.md)
- [Database Persistence](./database-persistence.md)
- [Local Proxy Runtime](./proxy-runtime.md)
- [External Agent Lifecycle](./external-agent-lifecycle.md)

## 2. Signatures and Layer Map

The default native call path is:

```text
renderer typed port
  -> invoke/adapter
  -> registered Tauri command
  -> service/domain owner
  -> DAO / filesystem / network / process / native adapter
```

### Command layer

- `src-tauri/src/commands/mod.rs` declares transport modules and re-exports the
  command functions consumed by `generate_handler!`.
- Command modules are private by default. A module may be `pub`/`pub(crate)`
  only when a current cross-module/test owner needs that surface and the
  architecture test admits it. `commands::skill` is an intentional current
  exception; it is not precedent for publishing all command modules.
- Registration, Rust export and Tauri permission are three separate changes.
  A command is not callable merely because its function is `pub`.

### Service/domain layer

- Services own orchestration, validation, transaction order, compensation and
  mapping from domain errors to command results.
- Commands parse/validate the wire request, obtain state, call one owner and
  serialize the result. They do not implement SQL, arbitrary path handling,
  HTTP clients, installer logic or provider document mutation.
- Reusable pure helpers belong in a domain/service module, not in a command
  file solely because the first caller is a command.

### Agent modules

The current `agent_install` family is intentionally split by responsibility:

```text
auth_actions.rs
auth_sessions.rs
cli.rs
desktop.rs
fetch.rs
inventory.rs
jobs.rs
lifecycle_policy.rs
macos.rs
sources/**
types.rs
windows.rs
```

Do not refer to retired generic names such as `auth` or `source`. Transport is
also split deliberately:

```text
commands/agent_catalog.rs
commands/agent_auth.rs
commands/agent_install_readiness.rs
```

Catalog/runtime, Auth sessions and lifecycle/jobs have separate Spec owners
and command façades. New work must extend the owning module instead of
recombining them into an `agent_install.rs` monolith.

### Proxy modules

`src-tauri/src/proxy/mod.rs` currently exposes several submodules publicly so
the crate's services, commands, providers and integration tests can share
types/routers/handlers. That visibility is intentional but bounded:

- `pub mod` inside this application crate is not a renderer API and does not
  authorize a generic Tauri command;
- external authority remains the registered command/permission set;
- moving a private Proxy implementation module to public requires a concrete
  in-crate owner and architecture-test update;
- command/service code still delegates listener/routing/protocol work to the
  Proxy owner described in [Local Proxy Runtime](./proxy-runtime.md).

### Database modules

- `database/mod.rs` owns connection lifecycle and shared helpers;
  `database/schema.rs` owns schema/migrations; `database/dao/**` owns SQL by
  feature.
- Services may compose DAO calls into transactions. Commands and renderer
  adapters do not lock `Database::conn` or issue SQL directly.
- A DAO module is not a second feature service: it maps rows and persistence
  errors; business legality remains in the service/domain owner.

## 3. Contracts

### Visibility is the minimum tested surface

- Prefer private modules/functions. Use `pub(crate)` for genuine crate-wide
  use and `pub` only for an intentional Rust public surface required by the
  current binary/integration-test architecture.
- Do not mechanically change every module to private: current Proxy and select
  command surfaces have real consumers. Conversely, one public module does not
  justify publishing siblings.
- Architecture tests are an allowlist/constraint, not generated documentation.
  Update the test only after the design identifies the new owner and why a
  narrower route is insufficient.

### One command, one owner

- Every renderer-callable operation has one registered command name and one
  typed frontend adapter.
- Avoid catch-all commands with operation strings, raw JSON, script names,
  filesystem paths, URLs or provider-specific arbitrary payloads.
- If multiple commands share a transaction, extract a service method; do not
  call one Tauri command from another.
- Error translation is deterministic. Do not collapse security, validation,
  conflict, cancellation and uncertain/rollback outcomes into one generic
  success/failure boolean.

### Side-effect ownership

| Side effect | Required owner |
| --- | --- |
| SQLite schema/version/SQL | `database/schema.rs` and owning DAO/service |
| Filesystem document transaction | owning service/native adapter with trusted paths |
| HTTP/vendor source | owning source/provider service with closed policy |
| Process/installer/native launch | reviewed native/lifecycle adapter |
| Secret resolution | native SecretService/SecretRef boundary |
| Proxy socket/protocol/failover | `proxy/**` + `services/proxy.rs` |
| Tauri state/event/window | command/app shell adapter, not domain core |

Pure domain functions receive values/traits and can be tested without
`AppHandle`, global state, network, filesystem or process execution.

### Registration and permission closure

- Adding/removing/renaming a command requires synchronized updates to:
  1. command implementation/export;
  2. `generate_handler!` registration;
  3. generated permission identifier;
  4. active application capability manifest;
  5. frontend adapter/parser and contract tests.
- The union of active application capability permissions equals the complete
  registered handler set. Feature-local permission additions must not silently
  remove unrelated commands.
- Remote origins remain absent unless a separately reviewed threat model and
  capability manifest explicitly admits them.

### No parallel legacy implementation

- V2 routes may coexist with leftover routes, but both must delegate to the
  same backend/service authority. Do not fork database, provider, Skill, MCP,
  Agent or Proxy behavior to make one UI easier.
- Compatibility exports/routes are thin adapters or routers. They do not retain
  a second transaction, schema, catalog or policy table.
- Delete unused private code after its callers move; do not expose it merely to
  avoid the move.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Renderer feature needs native operation | Add/reuse one typed command and service owner; no raw generic bridge. |
| Command contains SQL, vendor HTTP, archive extraction or process launch | Move side effect into DAO/service/native adapter. |
| New module is marked `pub` without an actual cross-module consumer | Keep private or `pub(crate)`; architecture review fails. |
| Existing Proxy/select command module is public | Preserve only the tested intentional surface; do not privatize mechanically. |
| New Agent code uses `auth`/`source` old owner names | Reject; use current `auth_actions`, `auth_sessions`, `sources` and correct command façade. |
| Command registered but permission/adapter missing | Contract failure; feature is incomplete. |
| Permission added but handler not registered | Contract failure; dead/wrong authority. |
| Capability manifest union drops unrelated handler | Reject even if the new feature works locally. |
| One Tauri command invokes another | Extract/call the shared service method instead. |
| V2 and leftover paths implement separate writes | Consolidate under one backend/service owner. |
| Domain helper requires `AppHandle` only to read config/emit UI event | Pass a narrow value/trait or move shell behavior to adapter. |

## 5. Good / Base / Bad Cases

- **Good:** add a typed command in the relevant command module, parse the
  request, call one service method, register/permit it and add a strict
  frontend port.
- **Good:** keep `proxy::types` visible because several in-crate owners share
  it, while retaining listener and routing behavior inside `proxy/**`.
- **Base:** a command module is `pub(crate)` for integration wiring. This is
  narrower than a public API and still requires no renderer authority beyond
  registered commands.
- **Bad:** mark every service/proxy module public, put `rusqlite` or `reqwest`
  logic in a command, expose `{ operation, payload }`, or create a V2-only
  native implementation parallel to the existing owner.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run check:contracts
```

Required assertions:

- `tests/architecture/rustModuleBoundaries.test.ts` reflects actual current
  private/`pub(crate)`/public surfaces and rejects unreviewed widening;
- command modules remain thin and no forbidden SQL/network/process imports
  move into transport owners;
- Agent module/path assertions use `auth_actions`, `auth_sessions`, `sources`
  and all three Agent command owners;
- handler registration, generated permissions and active capability manifests
  remain a complete disjoint union;
- integration tests call the public Rust surface intentionally rather than
  requiring accidental publication of unrelated internals;
- feature tests prove both V2 and leftover routes share the same native owner
  when both remain supported.

## 7. Wrong vs Correct

Wrong:

```rust
pub mod everything;

#[tauri::command]
async fn execute(operation: String, payload: serde_json::Value) -> Result<Value, String> {
    // SQL, HTTP, filesystem and process branching here.
}
```

Correct:

```rust
mod agent_auth;
pub use agent_auth::{get_agent_auth_observation, start_agent_auth_session};

#[tauri::command]
pub async fn start_agent_auth_session(
    request: AgentAuthRequest,
    state: State<'_, AppState>,
) -> Result<AgentAuthSessionSnapshot, AgentAuthErrorDto> {
    state.agent_auth.start(request).await
}
```

The command owns wire/state adaptation; the service/domain owner owns legality,
side effects, transaction and compensation.
