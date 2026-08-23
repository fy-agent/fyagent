# V2 Change Plan UI design

## Product and visual direction

- Existing FyAgent V2 Developer Tool / AI Product archetype.
- Preserve the established Blue Ambient / Clear Glass tokens and Models
  catalog geometry. Add no global token, font, shell, or navigation change.
- Use the existing sticky Models heading, Button, Dialog, InlineNotice,
  Spinner, and sanitized Provider summary. The change-plan workspace is a
  route-local operational surface, not a new dashboard shell.

## Ownership

- `src/v2/shared/features/change-plan.ts`: closed wire types, terminal helpers,
  safe presentation labels.
- `src/v2/shared/features/ports.ts`: one `ChangePlanPort` on `FeaturePorts`.
- `src/v2/shared/platform/tauri/feature-ports/changePlan.ts`: request guards,
  exact unknown-input parsers, invoke/listen ownership.
- Tauri/browser composition files: registration and native-only fallback.
- `src/v2/pages/models/change-plan/**`: Codex switch preview and job workspace.
- `src/v2/pages/models/Page.tsx` / `Page.css`: minimal route integration and
  product-matched styling.

## Runtime state machine

`idle -> planning -> preview -> applying -> terminal`

- `createPlan(targetProviderId)` supplies the immutable preview. Planning and
  preview do not call apply.
- Confirm subscribes before apply. The admission event reveals the job id; the
  event handler fetches the full snapshot and establishes polling.
- Polling is a fallback only after a job id is known. Event and poll snapshots
  are accepted only when `eventSeq` is greater than or equal to the current
  snapshot. Terminal snapshots stop polling.
- Apply rejection returns to preview with a controlled reason. A stale or
  expired plan must be regenerated; it is never automatically replayed.
- Unmount cleans the event listener and polling timer. No state enters URL,
  storage, or Query cache.

## Truth and safety

- The UI never renders `planDigest`, `baselineDigest`, IDs except sanitized
  Provider display names, raw `code` values as free-form messages, or resource
  paths. Stable enum/code values are mapped to reviewed Chinese copy.
- `snapshot` is described as binding the pre-write state, not as a guaranteed
  disk backup. `finalize` is described as closing the result, not refreshing a
  hidden cache.
- `usageEvidence=not_observed` always renders an explicit no-real-use-evidence
  statement, including success.
- The apply path calls only create/apply/get/cancel/listen; it never calls
  Provider fetch, reachability, or model probe methods.
