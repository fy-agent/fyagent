# Design — Agent lifecycle reliability program integration

## 1. Authority graph

```text
official source / native OS evidence
                │
                ▼
backend inventory + opaque candidate identity
                │
                ├── selected target + revision ──► platform deployer
                │                                  │
                │                                  └── authoritative readback
                │
                └── selected desktop target ─────► Auth handoff

official Auth observer ──► Auth observation ──► bounded session coordinator
                                                   │
                                                   └── verified / handoff-only /
                                                       failed / stopped / timeout

closed backend DTOs ──► feature ports + query ownership ──► shared V2 surfaces
```

No renderer path, command, URL, installer switch, credential or account identifier is an authority edge.

## 2. Child ownership

| Stage | Primary ownership | Must not own |
|---|---|---|
| 1 | Inventory identity, scope, revision, ambiguity and action target validation | Platform installer mechanics or page-local selection state |
| 2 | macOS source preparation, selected-path replacement, rollback and readback | Candidate selection or renderer-supplied destination paths |
| 3 | Windows evidence adapters, verified installer/helper boundary, elevation and readback | Generic Auth CLI execution or first-match discovery |
| 4 | Auth observation adapters and process-local session coordination | Installation job state, credential storage, Codex OAuth duplication |
| 5 | Route-derived navigation, shared UI primitives, feature/query lifecycle and chunk/warning gates | Backend authority, native installer execution or duplicate data clients |

The parent owns dependency ordering, evidence reconciliation and final disposition only.

## 3. Key integration invariants

### 3.1 Installation

- A visible candidate is not necessarily executable.
- Multiple trusted candidates are a user-selection state, never a collection-order decision.
- Update binds to the selected target revision and fails closed on drift.
- Platform process success is an intermediate event; authoritative installed-state readback is the success authority.
- Fresh-install fallback and in-place update scope rules are separate.

### 3.2 Authentication

- Handoff and verification are separate stages.
- Only a reviewed official observer can produce a verified outcome.
- OpenCode identity is provider-scoped; a global login boolean is invalid.
- Products without an observer remain handoff-only.
- Codex remains under the existing Auth Center.
- Backend session IDs are opaque lookup capabilities. Renderer reload recovery asks for the active session by canonical Agent ID; it does not persist execution inputs.
- Backend-process restart recovery is outside the implemented process-local store and remains an explicit gap.

### 3.3 Frontend

- URL/route identity is the navigation selection source of truth.
- Shared ports and query modules own native calls and cache lifecycle.
- Shared primitives own tab and assignment semantics.
- Route modules are lazy and measured by a checked-in verifier owned by CI classification.
- Unsupported actions are represented as absent/disabled state, not noop handlers.
- Browser/mock evidence proves renderer contracts and geometry, not installed native WebView behavior.

## 4. Validation layers

```text
unit/domain tests
      │
      ├── strict DTO/parser/transition tests
      ├── platform contract and security-boundary tests
      └── component/query/route tests
      │
      ▼
current-host full repository gate
      │
      ├── Rust format/check/Clippy/tests
      ├── frontend + V2 suites
      ├── task/lock/release/repository contracts
      └── desktop mock + visual preflight
      │
      ▼
four-viewport Chromium matrix
      │
      ▼
exact-head GitHub Full CI
      │
      ▼
signed-candidate native HIL (separate release gate)
```

Each lower layer may strengthen evidence but cannot substitute for a missing higher/native layer.

## 5. Failure and rollback policy

- Authority drift: refresh inventory/observation; do not reuse a stale target/session inference.
- macOS post-commit failure: restore and re-observe the old bundle.
- Windows elevation/installer/readback failure: return a stable incomplete/failed result without manufacturing an installed candidate.
- Auth observer failure or unsafe output: `unknown`/`unavailable`, never verified.
- Renderer reload: rediscover a process-local active session; if none exists, show current observation rather than guessing.
- Frontend query or route-load failure: explicit recoverable state; no static data fallback that could become a second authority.

## 6. Final disposition rule

The engineering program may be archived when child implementation/archives and integrated automated gates are complete. The final report must carry native HIL rows as unresolved release gates until exact signed candidates and disposable real accounts have been exercised.
