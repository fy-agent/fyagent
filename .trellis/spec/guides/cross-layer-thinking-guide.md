# Cross-Layer Thinking Guide

> **Purpose**: Think through data flow across layers before implementing.

---

## The Problem

**Most bugs happen at layer boundaries**, not within layers.

Common cross-layer bugs:

- Host returns format A, renderer expects format B
- Host stores X, renderer transforms to Y, but loses data
- Multiple layers implement the same logic differently

---

## Before Implementing Cross-Layer Features

### Step 1: Map the Data Flow

Draw out how data moves:

```
Source → Transform → Store → Retrieve → Transform → Display
```

For each arrow, ask:

- What format is the data in?
- What could go wrong?
- Who is responsible for validation?

### Step 2: Identify Boundaries

| Boundary                              | Common Issues                                      |
| ------------------------------------- | -------------------------------------------------- |
| Rust host ↔ Tauri IPC                | Command/DTO mismatch, missing registration         |
| IPC ↔ renderer port or API facade    | Each consumer re-parses the same payload           |
| Renderer ↔ React                     | Props/state ownership, event-listener lifetime     |
| Native window geometry ↔ renderer chrome | Treating host overflow and Overlay drag as one layer |

### Step 3: Define Contracts

For each boundary:

- What is the exact input format?
- What is the exact output format?
- What errors can occur?

---

## Common Cross-Layer Mistakes

### Mistake 1: Implicit Format Assumptions

**Bad**: Assuming date format without checking

**Good**: Explicit format conversion at boundaries

### Mistake 2: Scattered Validation

**Bad**: Validating the same thing in multiple layers

**Good**: Validate once at the entry point

### Mistake 3: Leaky Abstractions

**Bad**: Renderer knows about host storage schema

**Good**: Each layer only knows its neighbors

### Mistake 4: Every Consumer Parses The Same Payload

**Bad**: Each renderer consumer locally casts the same raw Tauri, event, or
configuration payload field.

This looks local, but it means every consumer owns a private version of the
payload contract. The next field change will update one consumer and miss
another.

**Good**: Decode/normalize once at the owner boundary, then export typed
projections to every consumer.

**Rule**: For Tauri commands, events, serialized DTOs, or config files,
create one owner for:

- event / payload type definitions
- type guards and normalization from `unknown`
- metadata projections used by UI commands
- reducers that replay state from the source of truth

Rendering code may format fields, but it must not redefine the payload contract.

---

## Checklist for Cross-Layer Features

Before implementation:

- [ ] Mapped the complete data flow
- [ ] Identified all layer boundaries
- [ ] Defined format at each boundary
- [ ] Decided where validation happens

After implementation:

- [ ] Tested with edge cases (null, empty, invalid)
- [ ] Verified error handling at each boundary
- [ ] Checked data survives round-trip
- [ ] Checked that consumers import shared decoders / projections instead of
      casting payload fields locally
- [ ] Checked that derived state uses the existing source version/cursor rather
      than inventing a second one
- [ ] Put concrete signatures, DTO fields, validation matrices, and test
      requirements in the owning backend/frontend code-spec, not this guide

For a Tauri command, event, or serialized payload, read
[Frontend Type Safety](../frontend/type-safety.md) and the owning backend
contract before changing either side.

When the change is native window geometry plus renderer chrome:

- [ ] Ask which layer owns host geometry versus Overlay/drag before editing either.
- [ ] Do not derive Overlay drag from `userAgent`, and do not shrink V2 layout
      to hide Windows maximize overflow.
- [ ] Put signatures and tests in
      [Main Window Layout](../backend/main-window-layout.md) and
      [V2 Shell](../frontend/v2-shell.md), not this guide.

---

## Version, Release Notes, and Archived Docs

Archived tasks and old versioned docs are historical evidence, not current
authority. Locate the owning backend code-spec and its `Tests Required`
section first:

- Application version and installer asset names:
  [Application Version and Installer Assets](../backend/fyagent-version-contract.md)
- Formal release notes and publication identity:
  [GitHub Release Workflow](../backend/github-release-workflow.md)
- Product identity and provenance exceptions:
  [Application Identity](../backend/application-identity.md)

Do not keep a parallel version/path matrix in this guide.

## Remote Status, Prefetch, and Endpoint Probe

When an installer, catalog, or configuration flow changes after a remote
status check or endpoint probe:

- [ ] Distinguish a definitive absence from a transient or malformed response.
- [ ] Keep retry/shortcut paths on the same validation and credential boundary
      as the interactive path.
- [ ] Reset stale cached/prefetched state when the selected source changes.
- [ ] Parse only complete bounded input; do not treat an arbitrary prefix as a
      finished response.

Exact URLs, DTO fields, error codes, and tests belong in the owning spec:

- [Codex Desktop Installer](../backend/codex-desktop-installer.md)
- [External Agent P0 Safety](../backend/external-agent-p0.md)
- [WorkBuddy Configuration](../backend/workbuddy-configuration.md)

## When a Cross-Layer Change Needs a Code-Spec

Update the owning `backend/` or `frontend/` code-spec (do not add a second
flow document here) when:

- The change spans host, IPC, and renderer
- The serialized shape or error matrix is changing
- The same boundary has already caused a bug

---

## Tauri IPC and Event Boundary

For a new or changed host-to-renderer payload:

- [ ] Map the Rust type, command registration, V2 port or legacy facade, hook
      or state owner, and rendering consumer before editing.
- [ ] Keep serialization/normalization at the owner boundary; consumers render
      typed projections instead of locally re-parsing raw payload fields.
- [ ] Decide which layer owns validation and structured errors, then test a
      success case, a rejected input, and an invalid/stale payload case.
- [ ] Keep event listeners bounded to the component/hook lifecycle and validate
      externally supplied event payloads before updating UI state.

The exact signature, DTO fields, error matrix, and test assertions belong in the
owning backend/frontend code-spec, not this thinking guide.
