# Issue 377 integration delivery

Draft PR: https://github.com/fy-agent/fyagent/pull/379

The branch integrates Codex account/catalog provenance, compact authentication
feedback and profile-aware omp configuration. Source behavior is frozen at
`77e46a70b`; `d10d6344e` removes redundant borrowed mapping keys without changing
behavior. The working tree has passed the canonical workspace/all-target strict
Clippy command after that cleanup.

## Verification

- Windows canonical native filters: managed_auth 198; omp main 34 and helper 4;
  agent_catalog 10; helper protocol 28; Windows helper parent 12. The same parent
  tests also pass in the integration target; do not add that duplicate coverage.
- Changed renderer scope: 322 cases have passing first-run/focused-rerun evidence;
  the final provenance change passed 55 focused cases. Typecheck, ESLint and
  production renderer build pass. Platform contracts pass 29/29.
- Chromium 900x600 and 1232x700: 60 scenario combinations have passing records.
  Final auth/provenance changes rechecked 24 combinations with one compact-scroll
  locator correction and a passing two-viewport rerun. Screenshots are browser
  fixtures, not native-window acceptance.
- Real Codex app-server returned seven models. Official omp 18.12.5 read the
  isolated production preview/apply output through config path, modelRoles and
  models find. No vendor inference was performed in that fixture.
- Helper process-lifetime fixture passed success/timeout child-tree cleanup and
  outside-sentinel preservation.

## Remaining acceptance

Keep the PR in Draft until native Windows window and packaged helper acceptance
are available. The host UI tooling stopped after a URL-policy identification
failure. The macOS native Codex catalog action returns platform_unsupported.

The broad catalog filter is not green: existing mainline mode-controller tests
lack WindowsInteractiveUserContext initialization, then panic again in Home
cleanup. All five involved files are unchanged from base 26c9295f0. The separate
ACL count integration defect is fixed and the relevant catalog checks pass.

The implementation task's draft-PR delivery is complete. These native/remote
acceptance items remain review follow-ups, not claims of release readiness.
