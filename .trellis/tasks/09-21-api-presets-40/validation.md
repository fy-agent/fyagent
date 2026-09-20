# #40 implementation evidence

## Scope and delivery

Branch `codex/next-api-presets-40`, base `2c09c4be`. No changes to the original
working tree, credentials storage, proxy/auth, navigation, release/version or
installer owners. Temporary `NEXT_ISSUES.json` / `VENDOR_SOURCES.md` are not
part of the commit. No vendor model calls were made.

- Shared ProviderPanel now has explicit protocol, actual production presets,
  user-initiated saved-source refill and per-product endpoint/key/use guidance.
- Optional closed protocol survives typed ports, native Quick Setup, existing
  Change Plan derivation, public DB summary and model probe request construction.
- Preset/saved-source refill clears state and ref Key, fetched models and old
  Codex features. Blank Key blocks preview even when a fixed saved row exists.
  Chat marks image migration complete so save preparation cannot enable the
  Responses-only feature behind the explicit protocol selection.
- Unknown native protocols/target combinations and known Alibaba product/key
  mismatches fail before writes. Generic discovery/probes refuse known plan
  endpoints and dedicated plan keys before network.
- Public connection readback includes only recognized endpoint/model/protocol;
  unsafe URLs, known credential collisions and unsupported shapes are omitted.
  This is DB Provider readback, not a current live-file or model-usage claim.

## Final checks (after relevant source changes)

PASS `mise run bootstrap` (frozen local dependencies; optional Windows
cross-build tool advisories remained, no new toolchain downloads).

PASS `mise run typecheck` and `mise run lint`.

PASS `mise run rust:fmt:check` and `git diff --check`.

PASS `mise run test:unit` with these eight filters: 8 files / 101 tests.

- `tests/renderer/pages/models/quickSetup.test.ts`
- `tests/renderer/pages/models/providerApi.test.ts`
- `tests/renderer/pages/models/Page.test.tsx`
- `tests/renderer/platform/providerApiPort.test.ts`
- `tests/renderer/platform/featurePorts.test.ts`
- `tests/renderer/platform/changePlansPort.test.ts`
- `tests/architecture/rustModuleBoundaries.test.ts`
- `tests/architecture/frontendModuleBoundaries.test.ts`

The first test run had 3 exact-payload expectation failures caused by the added
protocol field; those expectations were updated and final checks above pass.
Fixtures prove renderer/contracts only, not vendor entitlement or runtime use.

## Root integration / pending evidence

- Root explicitly owns all heavy native/build/browser work. No Rust compilation,
  native tests/Clippy, full frontend build/test, browser/performance or real
  vendor calls ran in this worktree. Suggested native filters: `provider_api`,
  `provider_draft_command_tests`, `model_probe`; existing Provider/Change Plan
  preservation/readback tests must also pass after #35 integration.
- `Page.tsx` save handlers are unchanged except adding `protocol` into the
  existing `validateQuickSetup` input. Preserve root's #56 single-confirm
  changes and #35 credential-copy updates when merging.
- #73 callback should reuse `fillApiForm({name, connection:{baseUrl, modelId,
protocol}})`, mapping endpoint/baseUrl and wireApi/protocol. This clears the
  previous Key and image/WebSocket selections. Do not turn a blank Key into
  implicit credential reuse based on fixed Quick Setup row ID.
- Saved-source fill is explicit copying into this page's existing Quick Setup
  configuration. Codex source activation still uses the existing Auth source
  workflow. Current live-file summary is #47, outside this package.
- Modern direct Codex clients may not accept Chat. The form states that an
  appropriate compatible client or existing conversion configuration is needed;
  this package neither installs/downgrades a client nor turns on a proxy.
- Ark Coding Plan Responses endpoint reuses the previously reviewed repository
  contract. Official doc re-fetch was unavailable; see `sources.md` for the
  exact evidence boundary.

Execution is complete; root acceptance and issue closure remain separate.
