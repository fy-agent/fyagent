# Cross-Layer Thinking Guide

Use this guide before changing data that crosses storage, native services,
Tauri IPC/events, renderer ports, state, or presentation. Exact behavior belongs
in the owning [backend](../backend/index.md) and
[frontend](../frontend/index.md) specs.

## Map the complete flow

Write the flow before editing:

```text
source -> validation -> native authority -> IPC/event -> renderer parser
       -> query/local state -> derived view -> user action -> mutation -> reread
```

For each arrow identify:

- input and output shape;
- semantic owner and authority source;
- size, count, lifetime, and concurrency bounds;
- secret/path/command/network exposure;
- failure, stale, cancellation, rollback, and unknown semantics;
- evidence required before the UI may claim success.

## Assign one owner per responsibility

| Responsibility | Typical owner |
| --- | --- |
| Storage/filesystem/process/network mutation | Rust service or platform adapter |
| Serialized DTO and closed error/state enums | Owning backend contract and Rust type |
| `unknown` wire parsing and normalization | Renderer platform/feature port boundary |
| Server snapshot and invalidation | Query hook/feature owner |
| Secret form draft | Component memory with explicit cleanup |
| URL selection | Router/search-param owner |
| Display-only derivation | Pure selector/view model |

Do not let the renderer know a native storage schema, let every consumer cast
the same payload, or let a process/HTTP success substitute for authoritative
readback.

## Define the contract before implementation

For a new or changed boundary specify:

- exact request/response and unknown-field policy;
- closed states/reasons and whether `null` means unknown;
- validation location and defense-in-depth checks;
- idempotency, locks, revision/capability binding, and cancellation boundary;
- rollback or partial-result semantics;
- redaction and what must never cross the wire;
- required unit, integration, renderer, browser, and native/HIL evidence.

Update an owning code spec when the change spans host/IPC/renderer, changes a
serialized shape or authority, or has meaningful security/platform failure
semantics. Do not put those details into this guide.

## Review the round trip

- Test valid, empty, null/unknown, malformed, stale, and oversized inputs.
- Verify storage -> read -> wire -> parser -> UI preserves intended data and
  rejects excess/sensitive fields.
- Verify mutation -> authoritative reread before positive copy or state.
- Verify failed/partial writes do not leave optimistic renderer state.
- Verify event listeners, queries, probes, jobs, and secrets have bounded
  lifecycle cleanup.
- Verify browser fixtures remain non-authoritative and native evidence is not
  inferred from portable tests.
- Verify version/path/history facts come from their owning configuration,
  provenance ledger, or Git history rather than a parallel guide matrix.

## Common failures

- Host emits one shape while a page assumes another.
- Multiple layers each normalize the same value differently.
- Renderer builds a URL, path, command, package identity, or capability that
  must be backend-owned.
- A launch, download, process exit, or endpoint response is painted as final
  installation/configuration/authentication success.
- A stale response overwrites a newer draft or selected target.
- A raw error exposes a credential-bearing payload.
- Native window geometry and renderer Overlay chrome are “fixed” in the wrong
  layer.
