# Implement — Stage 2

## 1. Preflight and specification

- [ ] Confirm Stage 1 inventory/target DTO is merged and no code still selects the first macOS root.
- [ ] Compare generic Agent DMG code with Codex Desktop macOS transaction and document the extraction/adaptation choice.
- [ ] Update external-agent and managed-package installer specs for exact-target update and no-silent-fallback semantics.

## 2. Shared macOS deployment

- [ ] Introduce/extend one private DMG mount and bundle replacement owner.
- [ ] Move Qoder/TRAE/WorkBuddy deployment off the direct `ditto`-to-user-Applications path.
- [ ] Pass Stage 1 candidate/destination capabilities, never renderer paths.
- [ ] Add same-parent staging/backup naming, symlink/file-kind confinement and bounded cleanup.
- [ ] Add commit-point cancellation transition.
- [ ] Add rollback and recovery-required outcomes.

## 3. Product adapters

- [ ] Implement bundle identity policy for all three products.
- [ ] Reuse TRAE `tronBuildVersion` and WorkBuddy version equivalence.
- [ ] Preserve selected target basename/path during update.
- [ ] Verify source/staged/installed bundle through the same product policy.

## 4. Authorization and runtime

- [ ] Add exact bundle-ID running observation or adapt an existing trusted runtime owner.
- [ ] Refuse unsafe force-close behavior.
- [ ] Review OS-native authorization alternatives; implement only a closed capability adapter with tests, otherwise expose an explicit authorization-required/manual state.
- [ ] Ensure update permission failure never invokes user-scope fallback.

## 5. Readback and UI

- [ ] Reread Stage 1 inventory after commit and compare target identity/scope/version.
- [ ] Verify no undeclared candidate appeared in another standard scope.
- [ ] Render preflight target/scope/version disclosure and terminal restored/recovery states.
- [ ] Use the shared target picker/status components; do not add product-local copies.

## 6. Tests

- [ ] Pure/fake filesystem transaction tests for fresh/update/rollback/concurrency.
- [ ] `/Applications` and `~/Applications` fixtures, including both present.
- [ ] Permission denied, symlink/replacement drift, multiple app in DMG, wrong bundle ID, malformed version and failed detach.
- [ ] Cancel before commit and reject cancel after commit.
- [ ] Qoder/TRAE/WorkBuddy post-install readback equality.
- [ ] Codex Desktop macOS regression suite.
- [ ] Native macOS HIL on disposable system/user targets and running-app scenarios.

## Validation

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Native HIL evidence must record pre-state, selected target label/scope, post-state, rollback result and whether elevation/runtime coordination was actually exercised.

## Rollback point

Do not enable product actions until the shared transaction and inventory verifier pass. If authorization remains unproven, ship target selection/readiness with system execution disabled rather than restoring the old user-directory fallback.
