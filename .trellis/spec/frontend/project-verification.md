# Project Verification Panel

`domain/verification/index.ts` owns portable DTOs, strict schemas and presentation
states. `shared/features/verification/VerificationPanel.tsx` exports the project
panel, composed by the project route owner; it creates no top-level route.
`VerificationPanel({ projectId, active? })` consumes FeaturePorts.verification.
Tauri parsing/command literals live only in feature-ports/verification.ts.

The Query key is featureKeys.verification(projectId). Automatic reads, expiry
timers and actions stop while hidden. Project changes remount scoped drafts;
late results update only their original project's query. Checks are explicit,
serialized by an immediate UI lock and native admission. Model checks disclose
possible usage. Each explicit click creates a UUID runId. The sample selector offers normal weekly input and a missing-field input, both closed native choices; internal checker identifiers are not product labels. Cancel is distinct from failed/successful checks. Failed reads
never present cached data as current authority.

Five stages remain independent. Expired/future/stale/revoked evidence cannot
display a green pass. Manual acceptance requires people, scope, time and actual
external references or valid basis IDs. UI allows genuine external customer
records without requiring a local machine pass, while the native service refuses
fixture-only bases. Source labels always distinguish local synthetic validation.

Handoff edits save structured issues, nullable owners and closed rollback choices
with CAS. Preview and JSON must match a strict parsed snapshot. Export is a
native chosen-file operation; no renderer-provided filesystem path or payload is
accepted. Native failures use fixed safe copy, never arbitrary rejection text.
Browser-only ports reject native operations, without production mock success.

Tests: verification.test.ts, VerificationPanel.test.tsx, verificationPort.test.ts,
tauriAclContract and renderer architecture. Check cross-project results, extra
secret fields, machine customer acceptance, TTL, manual external acceptance,
hidden dispatch, independent stages, preview/JSON consistency, cancelled export
and safe errors. Native fixtures/runtime are separate evidence layers.
