# Project Verification and Handoff

## Ownership

`services/verification/` owns durable five-stage facts, validity, manual acceptance,
and redacted handoff. `database/dao/verification.rs` owns local-only evidence,
revocations and handoff rows. `commands/verification.rs` is the closed IPC/dialog
boundary. This domain never queries/writes project tables or invokes Health auth
reconciliation. Composition injects `ProjectDependencyReader`; the default
unavailable implementation cannot produce successful project observations.

## Native reader and checks

`ProjectDependencySnapshot` binds projectId/projectRevision, optional kitId,
kitVersion/manifestDigest, active state, resource revision, credential generation,
configuration-saved fact and independent projection status. Revisions must be
monotonic, including A→B→A. Resource changes, deleted projects, changed kit or
credential generation invalidate previous facts; missing authority is unverifiable.
The renderer supplies project ID, expected revision, UUID runId and closed checker/fixture selections. It never supplies outcomes, raw inputs or paths.

Registered checks are saved configuration readback, saved-model probe and a
kit-owned local synthetic validator. The model reader resolves a saved binding
natively. Secret strings are non-serializable, have no Debug and zeroize on drop.
Missing credential generation prevents a request from being accepted as verified. None means unknown tracking; a known credential-free binding should have its own native generation. External manual records can still be registered, but non-configuration/non-fixture records remain unverifiable while tracking is unknown.
`model_probe::probe_saved_identity` reuses the existing request projection and
HTTP owner, requires exact response model identity against the same request projection (including removal of explicit reasoning-effort suffixes), plus actual text output in a
bounded stream, caps time at 30 seconds and body at 64 KiB. Unknown aliases and
HTTP 200 without matching identity/output do not pass. The existing draft probe
retains its historical connectivity semantics. No arbitrary MCP process executes.

Kit validation consumes a native kit-owner receipt with exact identity/version/
digest and selected fixture. Baseline/missing_field map to kit fixture baseline/missing-field. Store synthetic input digest, closed business code, expectation match and fixed weekly-report metrics (current/previous minor units and growth/target basis points), plus the six built-in source row IDs. Integers remain within the JavaScript safe range; failed business input has no metrics. Negative input remains a failed business result even when rejection was expected. Its source remains local_fixture. A test-injected reader is not an IPC
capability and must never be installed as a production-success fallback. UI/Markdown preserve concrete closed business failure reasons.

## State and history

configuration_saved, authentication_available, tool_callable, sample_passed and
customer_accepted are independent. Outcome and current/stale/revoked/unverifiable
are separate. Only manual records can be customer acceptance. Every record has
source, observed/recorded time, native dependencies, checker/application version,
safe reason and optional basis IDs. Records append; revocations retain original
facts. A new UUID runId creates a new observation; persisted replays return the existing snapshot without rerunning. Reusing a runId with another project/revision/checker/fixture is rejected. Process-local per-project
admission prevents simultaneous checks. A later non-pass for the same checker, stage, fixture and dependency snapshot makes earlier passes stale, including for manual basis admission; distinct fixtures remain independent. Cancel records cancelled without treating
an interrupted network request as passed. Read-only native work already started
may finish after cancellation, but no success is persisted from that work.

Native dependencies are read before and after execution. Failure to persist is
evidence_persist_failed, not a retry instruction. Auth/tool facts expire after
15 minutes; future timestamps or clock rollback are unverifiable. Basis validity
propagates into manual acceptance. Manual external acceptance needs person, role,
scope, observed time and a checkable external issuer/reference, or valid real
basis IDs. Local machine pass is not compulsory when genuine external evidence
is manually registered. Fixture-only evidence cannot be laundered through a
manual intermediate record. A basis cannot occur after its asserted acceptance.

## Persistence and exports

The additive domain migration is `Database::migrate_verification_v22`; root owns
composition into the single schema version transition. Three tables are omitted
on sync export and locally preserved on sync import. Deferred foreign keys keep
revocation/evidence restoration atomic. Handoff updates have independent CAS
revision. Data import/restore retains the database owner's validation boundary;
record reads additionally validate safe public fields before projection.

Public DTOs/export exclude native resource fingerprints, credential generations,
SecretRef, raw errors/responses/configuration and recovery capabilities. Human
fields are bounded structured labels, not arbitrary documents. External references additionally admit bounded HTTPS document URLs without userinfo, query, fragment, encoded/control characters or secret markers; other human fields stay label-only. No reference URL is fetched. Default
exports contain these explicitly previewed people/scopes/issues and fixed rollback
limits. JSON and Markdown derive from the same current snapshot; native file save
dialog selects destination, and dependencies are reread before a private reversible
write. A file undo does not undo account grants, DB state or remote business work.

## Checks

The composed sample/context/export workflow requires the macOS project-files
owner and runs only on macOS. Windows keeps the file owner explicitly
unavailable: its composition test requires `projects_platform_unavailable` from
context writing, unchanged project revision/generation, no published context and
no project directory. Cross-platform checks such as unbound-package rejection
remain enabled on both platforms; do not skip the whole FDE test module or turn
an unavailable file operation into successful evidence.

Run `mise run rust:test -- verification`, `mise run rust:test -- model_probe`,
relevant migration/sync tests and Rust Clippy/fmt. Required cases: config cannot
become customer pass, external manual acceptance, fixture laundering refusal,
A→B→A and cross-project/kit isolation, credential rotation, TTL, clock rollback,
revocation propagation, concurrent changes/cancel, migration rollback, local-only
sync, loopback identity/401/timeout and secret-free exports. Fixtures do not prove
real external services, customer acceptance, native UI, or a release.
