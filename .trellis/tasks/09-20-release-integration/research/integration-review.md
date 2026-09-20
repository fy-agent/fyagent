# Combined FDE and subscription integration review

Status: approved for the bounded semantic integration/source review after the
backend and frontend owners froze their changes. No actionable review finding
remains open. The root-owned unified gate and remote CI remain separate gates;
this approval is not a release, Windows native or real-account UAT claim.

Scope: semantic interaction of FDE `6f38c213` and subscription `6a4af16d` in
the existing integration worktree. This review does not modify production code,
stage commits, run Cargo or duplicate the owners' renderer/browser suites.
Only source and fixture evidence are considered; no real-account UAT is claimed.
Backend paths below are relative to `src-tauri/src/` unless otherwise specified.

## Findings and closure

### IR-1 — Saved-model verification treats a managed marker as an upstream key (P1)

- Evidence: `services/fde_workspace.rs` resolves its saved provider through
  `ProviderService::extract_credentials`, then `verification/mod.rs` sends those
  values through `probe_saved_identity`. The managed OpenCode/Grok builder stores
  the upstream subscription URL and `PROXY_MANAGED`, while the real loopback URL
  exists only in the target live projection. The direct reader consequently
  bypasses the managed transport. Claude/Codex managed rows lack an ordinary key
  and already produce an unsupported result.
- Required closure: root selected the minimal capability boundary. Reject all
  managed subscription providers in the FDE saved-model reader before creating a
  network request; retain `unsupported / saved_model_unavailable`, not a vendor
  request failure. Test both subscription sources for the four supported targets.
- Fix readback: the backend owner added `uses_subscription_proxy()` admission
  before credential extraction. The new workspace regression runs both sources
  across four targets through the production verification service, asserts the
  unsupported reason/validity and checks that a loopback listener received no
  request. Static closure accepted; the compiled regression passes in
  `backend-release-integration-final.log`. The unsupported capability boundary is
  also explicit in `.trellis/spec/backend/project-verification.md`.
- Scope boundary: target-Agent subscription reuse remains supported; automatic
  FDE saved-model verification of that subscription is not added by this merge.

### IR-2 — OpenCode startup import overwrites managed binding authority (P1)

- Evidence: `lib.rs` automatically calls
  `import_opencode_providers_from_live` before startup proxy recovery. The import
  in `services/provider/live.rs` replaces an existing row's `settings_config`
  with the live fragment. For a managed subscription that replaces the stored
  upstream URL with a loopback projection. A subsequent bind compares the row
  with the stable builder definition and returns `provider_conflict`.
- Required closure: preserve existing managed provider rows and refuse importing
  owned/reserved proxy projection fragments as ordinary providers, while retaining
  raw vendor fields for ordinary OpenCode imports. Cover bind, import, repeat bind
  and recovery in a shared fixture.
- Fix readback: import now skips reserved IDs and proxy-marker fragments and
  preserves existing rows with managed metadata. The generic OpenCode writer
  also refuses managed providers. The existing bind fixture now imports the live
  configuration, compares the saved settings/metadata and native auth bytes, then
  repeats binding and restore. Both the new import/generic-writer regression and
  the combined bind/import/generation/rebind/restore regression pass in the final
  integration and subscription logs.

### IR-3 — OpenCode Health reports intentional proxy projection as endpoint drift (P2)

- Evidence: `services/health/proxy.rs` returns `not_supported` immediately for
  OpenCode. `health/configuration.rs` therefore compares the stored subscription
  upstream endpoint with the correct `/opencode/v1` live loopback endpoint without
  applying the existing proxy projection correction.
- Fix readback: the Health observer recognizes `/opencode/v1`, checks the current
  managed provider ID, and derives the expected single model from its saved binding.
  The new fixture covers the intact route, wrong path/port/model/provider, stopped
  listener and unchanged database/native-file bytes. It does not substitute a
  native OpenCode login for the selected proxy binding. Static closure accepted;
  the compiled regression passes in the final integration log. The first run
  exposed the omitted OpenCode SELECT-only Health DAO whitelist; that correction
  is included in the passing run.

### IR-4 — Read-only proxy-slot projection drops the target boundary (contract item)

- Evidence: the merged `observe_overview_inner_with_proxy_observer` filters all
  OpenAI/xAI `fyagent_proxy` slots, while the persistence pruning owner and
  `managed-account-proxy.md` restrict that cleanup to `target_id = ''`.
- Recommendation: apply the same empty-target restriction to the read-only
  projection and preserve records owned by another target. Current public binders
  create empty-target slots, so this is a compatibility/spec discrepancy, not
  evidence of a real-account outage.
- Fix readback: the projection now restricts cleanup to empty-target standard
  OpenAI/xAI credential slots, matching the DAO boundary. Static closure accepted.

### IR-5 — Read-only route observation stops before a valid later target

- Evidence: the FDE read-only selector returned `None` immediately for a stale
  selected provider. Adding OpenCode as the fourth target made an unrelated stale
  Claude selection hide its otherwise proven route.
- Fix readback: unreadable selections accumulate an unknown result while the
  observer continues. A positive result still requires the same auth kind and
  account, an owned running loopback listener and the matching target's live
  projection. Without a positive match, unreadable state remains `None`.
- OpenCode compares the captured provider directly, avoiding the lifecycle helper
  that can repair a stale selection. Its comparator checks the exact provider ID,
  complete projected provider value and selected model. No route evidence from a
  different account is substituted.
- Tests: the two-source OpenCode fixture proves an unrelated stale Claude ID does
  not hide the selected account's route and remains unchanged after observation;
  the transport fixture retains stale-only `None`, stopped/unadopted results and
  unchanged database/settings evidence. The final subscription filter, including
  the captured-provider adjustment, passes all 51 tests in the verified log.
  The shared test-home helper reloads process settings after both setting
  and restoring its isolated environment, resolving prior fixture leakage.

### IR-6 — First-use guide inherits no detail icon dimensions (root finding)

- Evidence: `FirstUseGuide` renders `BrandIconFrame` outside the catalog containers
  that define its detail-size CSS variables. The added regression failed before
  the fix with an actual 512 px frame against the expected 64 px.
- Fix readback: `src/pages/agents/FirstUseGuide.css` defines the existing detail
  frame/artwork variables as 64 px / 48 px on the guide itself. Shared catalog
  sizing and budgets are unchanged. The browser regression checks frame and
  artwork dimensions while preserving viewport, scrolling and completion checks.
- Tests: read back the root-owned before/after logs; the corrected first-use suite
  passes 30 tests. This reviewer did not rerun browser tests.

## Reviewed paths without an additional finding

- Frontend facade forwards ordinary model operations, managed bind/model lookup
  and OpenCode restore through the deferred model port without dropping arguments.
- OpenCode editing uses exact provider IDs, excludes reserved subscription rows,
  retains builtin read-only eligibility and blocks ordinary writes while a managed
  projection is present. The combined renderer regression restores a managed
  projection and saves to the second of two same-name ordinary providers by ID.
- The restore controller retains the parent write lock through both authoritative
  snapshot and account-overview reads; missing readback blocks subsequent writes.
- Managed Auth management reconciliation and Health share per-credential proxy
  projection; the Health admission path calls the read-only facade. Account counts
  derive from projected wire connections, including independently selected accounts.
- The schema forward step completes OpenCode and FDE structures for either prior
  v22 shape, under the existing savepoint. The fixture preserves all prior proxy
  columns, FDE records, tombstone generations and evidence, repeats migration,
  checks trigger increments, denies the final version write to test rollback and
  tests binary restore without modifying the selected backup.
- Sync skip/preserve lists contain both Managed Auth local metadata and the FDE
  project/evidence tables. Binary restore prepares migration and generation
  advancement in a candidate before replacing the live database.
- The route guard models the actual two-level split: exactly nine routes and six
  bootstrap ports, plus the sole `models -> managedSubscriptions` dynamic entry.
  All seven ports remain excluded from initial static imports, and subscriptions
  are also excluded from the Models static closure. Existing 650 KiB total JS,
  300 KiB initial chunk, 64 KiB CSS and 180 KiB route/port budgets are unchanged.
  New negative tests retain rejection of missing/extra dynamic entries, leaked
  static dependencies and a missing/shared Projects route chunk.

## Additional CI closure review

- The FDE tree's Windows test changes initialize the same frozen Shell-user
  context that production `main` initializes. They do not change the production
  identity resolver. Agent test logs retain a temporary root; Health fixtures
  retain isolated configuration and secret storage, while installation inventory
  may read platform evidence. Actual Windows runner initialization still requires
  Windows CI evidence.
- Windows project context writes already return `projects_platform_unavailable`
  before file or database publication. Gating the macOS materialization scenario
  and adding a Windows test for unchanged project/revision, empty `NotCreated`
  context and no project directory matches that existing capability boundary.
- Closed integration adaptation: in the FDE tree, `Validity` is only used in the
  macOS fixture and can have a macOS import gate. The combined tree's new
  `release_integration_fde_saved_probe_rejects_subscription_sources_before_transport`
  uses it on both platforms. When transferring the patch, keep `Validity`
  unconditionally imported; only `SourceClass` and `BindKitRequest` need the macOS
  gate. Root's final combined source preserves this distinction; readback accepted.
- The `tests/browser/blue-themes.spec.ts` sampling guards wait for the Agent
  directory's `complete` state (the only state labelled `重新扫描`) and removal of
  the progress block. The same reducer transition commits the card order. This
  precedes geometry capture and leaves colors, sample selection and the 4.5
  threshold unchanged. It addresses the identified scan/layout race without
  weakening assertions. The CI failure itself records 4.094 contrast but does not
  prove the geometry changed between capture and screenshot; the race attribution
  remains a source-supported explanation. Both final contrast cases pass across
  all five browser/viewport configurations in the 19-test regression log.
- The shell count now counts controls in the top bar/navigation, matching its
  intended scope. The test still issues all eleven real Tab steps and compares the
  entire focused-control sequence; it does not skip an unexpected tab stop. A
  native-only content retry button no longer inflates the separate shell count.
- The theme test captures its real native track through browser animation frames
  and immediately reverses it before the 560 ms window can be missed by separate
  protocol polls. It retains exactly one track, active marker, 560 ms duration,
  easing, circular clip, original pointer origin, reversed theme and resize/cleanup
  assertions. No product animation timing changes. The final targeted run covers
  all supported browser/viewport configurations; the pre-existing WebKit keyboard
  exclusion was not changed.

## Verification status

- Static review: completed for the paths above and the six findings. Root relayed
  backend's formal freeze; the final owner report confirms it. Frontend and the
  final CI regression-test source are also frozen. No further source changes were
  requested by this reviewer.
- Renderer focused tests: read back
  `.trellis/.runtime/verification/integration-frontend-targeted-final.log`:
  five files, 138 tests passing. Also inspected the combined restore/exact-ID test.
- Frontend lint, TypeCheck and format: owner reports successful exit; read back
  the corresponding `integration-frontend-{lint,typecheck,format}-final.log` files.
  Root still owns the final unified gate after backend freezes.
- Rust tests: directly read `backend-release-integration-final.log` (7 passed) and
  `backend-release-subscription-verified.log` (51 passed after the last
  captured-provider adjustment). Also read `backend-release-health-final.log`
  (43 passed), `backend-release-config-reliability.log` (7 passed) and
  `backend-release-database.log` (117 passed / 2 ignored, plus one matching
  import/export integration test passed). The root owns the unified Rust gate.
  Earlier fixture compile errors, the omitted Health DAO target and cached
  settings failures are superseded by these passing focused runs. Older
  subscription-branch logs are not counted as combined-head evidence.
- Browser: root-owned `guide-icon-regression-before.log` reproduces the missing
  dimension and `guide-icon-regression-after.log` reports 30 passed. Full browser
  `integration-browser-final.log` reports 638 passed / 2 failed (shell keyboard
  path and theme reveal timing). The final source fixes and contrast guards pass
  all 19 tests in `ci-browser-regressions-final.log`, which this reviewer read back.
  This focused closure does not relabel the earlier full run as passed; the root
  owns the full gate after the final changes.
- Real-account UAT, Windows/native acceptance and release: not performed here.
