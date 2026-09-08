# Owning spec constraints

The executor must read each complete live spec before implementation. This summary prevents context truncation; it is not a replacement.

## Backend

### `.trellis/spec/backend/reuse.md`

- Existing owner before new dependency or implementation.
- No duplicate downloader, filesystem transaction, process runner, helper or IPC protocol.
- Dependency additions require explicit review.
- Negative duplication scans are acceptance work.

### `.trellis/spec/backend/modular-boundaries.md`

- Commands stay thin.
- Product/platform code stays behind crate-private services/facades.
- Platform modules must not become alternate application services.
- Dependency direction remains acyclic.

### `.trellis/spec/backend/external-agent-p0.md`

- Stable seven-product catalog and one legal surface/action owner.
- Opaque inventory/target/release capabilities.
- Windows evidence adapters and exact executable/package identity.
- Desktop source/job/launch semantics.
- Grok is the only CLI lifecycle exception.
- Other product/config domains remain independent from installation surface.

### `.trellis/spec/backend/windows-runtime-security.md`

- Freeze Explorer interactive-user authority.
- Registry/profile/PackageManager/CLI decisions use the same principal.
- Helper IPC is closed and nonce/SID/session/image/action bound.
- No renderer command/path authority and no elevated fallback.
- UAC/system operations fail closed.

### `.trellis/spec/backend/codex-desktop-installer.md`

- Exact package identity/PFN/AUMID/runtime authority.
- PackageManager/deployment/source/post-readback contracts.
- Helper/restart safety and no name-based kill/launch.
- Any ChatGPT/Codex migration is exact and atomic across inventory/install/update/launch/restart.

## Frontend

### `.trellis/spec/frontend/reuse.md`

- Shared product metadata/components before page-local branches.
- Backend `allowedActions` is authoritative.
- No duplicated platform/product action tables.

### `.trellis/spec/frontend/v2-agent-models.md`

- Directory/readiness/lifecycle card contract.
- Stable product IDs and physical component labels.
- Scan/stale/ambiguous states and accessibility.
- Strict parser/action projection.

### `.trellis/spec/frontend/user-facing-copy.md`

- Concise stable reason copy.
- No raw paths/URLs/SIDs/package IDs/certificates/installer output.
- Platform, architecture, scope, manual fallback and unsupported states are honest.

## Task-specific contracts to add after implementation

- Desktop-only on both platforms except Grok Build CLI.
- All non-Grok Tooling install/update/manual-command surfaces retired; read-only/config consumers preserved only when needed.
- Formal Windows Grok ordinary-user helper with a closed product/action protocol.
- Qoder/TRAE/WorkBuddy remain install-only.
- Claude/OpenCode Windows package/source/identity policies.
- Exact ChatGPT/Codex/Classic verification and evidence-gated migration.
- Expected retryable Codex deferred is not routine WARN/INFO; retry/cursor/replay correctness remains authoritative.
