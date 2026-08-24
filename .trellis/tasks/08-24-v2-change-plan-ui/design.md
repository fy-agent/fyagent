# Canonical V2 Change Plan UI design

## Ownership

React Models Apply remains a projector, not an executor. The only write is
`changePlans.applyChangePlan({ planId, planDigest })`. Job truth comes from
validated `ChangeJobSnapshot` values. Do not add a second state machine, fake
progress timer, scenario coordinator, or page-local DTO type.

Keep presentation helpers in
`src/v2/pages/models/apply/view-model.ts` and
`src/v2/pages/models/apply/ApplyWorkspace.tsx`. Polling/request revision stays
in `ChangePlanWorkspace.tsx`, which already owns `requestRevision`.

Do not create a shared chrome primitive for this four-section preview unless a
second current consumer exists. This surface is Models Apply only.

## Preview contract

Map the existing closed `ChangePlan` fields into four sections. No extra wire
fields.

| Section | Sources | User-visible rule |
| --- | --- | --- |
| 语义变化 | `currentProviderCode`, `targetProviderCode`, `targetProviderName`, `operation` | State the switch in Chinese; no raw digest as the primary explanation |
| 风险与重启 | `risks[]`, `restartExpectation` | Empty risk list is an explicit “无额外风险项”, not a missing section |
| 前置条件与范围 | adapter `readSet`/`writeSet`, `secretCapability`, baseline provider ids, expiry | Fail closed if `secretCapability=secret_dependency_unavailable`; no confirm |
| 恢复方式 | `evidenceNote`, compensation mode, readback-only recovery copy | Promise writer-owned rollback and no write replay, not a second undo engine |

Preview rendering must not call apply, cancel, or any Provider writer.

## Execution refresh

Current apply path:

1. `applyChangePlan`
2. one `getChangeJob`
3. stop

Replacement:

1. keep the same apply call and idempotent_replay handling
2. while the latest requested job is `planned` or `running`, poll
   `getChangeJob(jobId)` on a bounded interval (1s is enough; no backoff
   theatre)
3. ignore stale responses via `requestRevision`
4. stop on terminal status, workspace close, target change, or unmount
5. do not start a second apply to “refresh”

Do not subscribe to a renderer-owned event bus. Backend event hints remain
`{jobId,eventSeq}`; if a live event port already exists, it may invalidate the
current job query, but polling is the required fallback.

## Partial truth and sequence

`createApplyViewModel` already classifies terminal copy. Extend it to expose:

- `partialResult` projection when non-null
- `eventSeq` plus the already-parsed `events` list if useful as secondary detail
- no local derivation of succeeded/compensated/unverified from step order
  beyond what the DTO already states

Manual actions stay in the closed enum: `retry_readback`,
`review_configuration`. Do not render raw error strings from outside the DTO.

## Cancellation

Port `cancelChangeJob` remains. Models Apply does not grow a cancel button,
busy=`cancelling` state, or optimistic cancelled snapshot. Backend-cancelled
jobs still use the existing confirmed pre-write copy.

## Compatibility

- Do not change schema v20, SecretRef, or Codex Quick Setup targeted patch.
- Do not rename `change-plans` back to `change-plan`.
- Browser `changePlans` port stays `rejectNativeOnly`.
- Recoverable-job notice remains read-only readback; still no write replay.

## SPEC

Update `.trellis/spec/frontend/v2-agent-models.md` Agent-readiness / Change
Plan UI section:

- require the four-section preview
- require bounded `getChangeJob` polling
- require partial-result / eventSeq presentation
- keep “no Models cancel button”

## Explicit non-goals

- Codex create/edit/switch vertical
- WorkBuddy adapter
- restoring Draft #136 CSS/page tree (`pages/models/change-plan/`)
- expanding backend preview payload
