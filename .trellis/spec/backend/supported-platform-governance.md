# Supported-Platform Source and Asset Governance

## 1. Scope / Trigger

Read this contract before changing `supported-platform-check.mjs`, its
first-party source/raster identity inventories, the whole-repository scanner
integration test, or any platform guard that changes which tracked files are
considered platform-sensitive.

This owner protects review identity and scanner completeness. It does not own
the implementation behavior inside each listed file, product platform support,
or native acceptance. The public task entry points are documented by
[Repository Task Runner](./task-runner-contract.md). The temporary active-task
exclusion used only before Trellis archive is documented by
[Trellis Direct-Session Prearchive Gate](./trellis-prearchive-gate.md).

## 2. Signatures

```text
mise run supported-platform:check

node scripts/tasks/supported-platform-check.mjs
  [--exclude-active-task <validated canonical path>]

scripts/tasks/supported-platform-structure-assets.json
  -> reviewed platform-sensitive first-party source identities

scripts/tasks/supported-platform-raster-assets.json
  -> reviewed tracked raster identities

identity entry = {
  path: canonical repository-relative path,
  mode: reviewed Git index mode,
  sha256: lowercase file digest
}
```

The checker is dependency-free and runnable from a clean checkout with Node
built-ins only. CI Changes invokes it before dependency installation.

## 3. Contracts

### Complete source and asset inventories

- The two inventories are fail-closed review authorities, not content
  exclusions. Every listed file still passes normal path, text and structural
  scanners.
- The source candidate set is recomputed bidirectionally from all tracked Cargo
  manifests/build scripts and executable/configuration files containing
  reviewed platform selectors. Candidate path, Git index mode, regular
  non-symlink file type and SHA-256 must match exactly.
- Platform-sensitive source is normally `100644`. The Tauri Cargo runner
  `scripts/tasks/macos-signed-dev-cargo.mjs` is deliberately `100755` because
  Tauri executes its `--runner` directly; changing that mode breaks runtime and
  is not a hardening improvement.
- Adding, removing, renaming, moving, editing or changing the mode of a
  candidate fails until the final bytes and platform dispatch are reviewed and
  the inventory is deliberately updated. A digest-only refresh is not review.
- Recompute after formatting or any later source edit. A manifest made green
  before final formatting is stale evidence.
- Canonical manifest order is `path.localeCompare(other, "en")`, not raw byte
  order. Bulk-refreshing unreviewed entries or disabling the checker to unblock
  CI is prohibited.
- A platform guard added to a previously neutral module creates a new
  candidate. Add and review it rather than introducing an exclusion.

### Whole-repository snapshot validation

- The integration test captures the tracked Git file list once, then runs
  every scanner against that same immutable snapshot. No scanner may silently
  enumerate a different repository state during the same assertion set.
- The repository currently contains more than 1,000 tracked files. The
  integration test has a bounded 15-second test watchdog for enumeration and
  parsing scheduling; that watchdog is not a product latency or scanner
  performance budget. Other tests retain their normal timeout.
- Zero findings and inspected-file-count assertions are mandatory. Negative
  fixtures for each inventory/scanner remain required.
- Do not change the production scanner, suppress findings, or loosen a product
  performance budget to accommodate the integration runner's scheduling.
- When repairing a source-text guard, assert the complete owning platform block
  and add negative fixtures for moving or widening the protected operation.
  Attribute/call adjacency alone is not an authority boundary.

### Runtime and CI boundary

- The checker imports only Node built-ins and dependency-free repository
  bootstrap helpers. The always-running CI Changes job executes it immediately
  after checkout and Node setup, before package installation.
- GitHub-hosted Linux execution proves the repository scanner and inventories,
  not a shipped Linux product surface or native Windows/macOS behavior.
- Default local/CI checks never infer an active-task exclusion. Only the
  separately validated prearchive wrapper may provide the private path, and the
  post-archive canonical run removes it.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Candidate set differs from manifest in either direction | Fail with added/removed identities. |
| Tracked mode, regular-file type or SHA-256 differs | Fail; require review and explicit inventory update. |
| Candidate or inventory entry is a symlink/escape | Reject. |
| Manifest order differs from English locale ordering | Fail canonicalization. |
| New platform guard is hidden by an exclusion | Contract regression; admit and review the candidate. |
| Integration scanners enumerate separate live file lists | Test failure; use one captured Git snapshot. |
| Whole-repository scan exceeds the bounded integration watchdog | Diagnose scheduling/fixture cost; do not weaken production scanner or product budget. |
| Zero-findings or inspected-count assertion is removed | Contract regression. |
| Checker imports an installed package before CI dependency setup | Changes job failure; restore dependency-free boundary. |
| Canonical check receives an inferred/private task exclusion | Reject; only validated prearchive may exclude one exact task. |
| Portable scan is cited as native runtime/install/signing evidence | Keep matching-host evidence pending. |

## 5. Good / Base / Bad Cases

- **Good:** a reviewed platform branch is added, the complete source bytes are
  inspected, the new candidate is inserted in canonical order with final mode
  and digest, and every scanner passes against one Git snapshot.
- **Good:** a source-text assertion is widened to the entire authority block
  and negative fixtures prove the protected call cannot move outside it.
- **Base:** an ordinary docs change still runs the dependency-free checker in
  the CI Changes job and finds the same complete repository identities.
- **Base:** the whole-repository integration test needs up to its dedicated
  watchdog under host load while individual scanner/unit budgets stay intact.
- **Bad:** refresh every digest without reviewing the diff, sort with raw bytes,
  mark the executable Tauri runner `100644`, or skip an added platform file.
- **Bad:** raise application performance thresholds or suppress findings
  because a repository integration test enumerates many files.

## 6. Tests Required

- Bidirectional fixtures cover added, removed, renamed, moved and newly
  platform-sensitive source, plus mode/digest/file-type/symlink drift.
- Raster fixtures cover tracked identity, content and mode drift.
- Ordering tests require `localeCompare(..., "en")` output and reject raw-byte
  alternatives.
- Whole-repository validation captures one Git file list and asserts every
  scanner's inspected count plus zero findings within the dedicated watchdog.
- Negative source-text fixtures move or widen protected operations and must
  fail even when an attribute/call token remains adjacent.
- Clean-checkout tests prove the checker uses only Node built-ins and can run
  before dependency installation.
- `tests/remainingPlatformSurface.test.ts`, the dedicated checker tests, and
  `mise run supported-platform:check` remain green.
- Prearchive tests prove exactly one validated active-task path may be excluded;
  canonical local/CI/post-archive tests prove no exclusion is present.

## 7. Wrong vs Correct

### Wrong

```text
source changed -> bulk regenerate every SHA -> commit
new `cfg(target_os)` file -> add ignore pattern
integration timeout -> remove inspected-file assertion
```

### Correct

```text
review final source bytes and dispatch
  -> recompute exact candidate set
  -> verify mode + regular file + SHA-256
  -> insert by localeCompare(path, "en")
  -> run all scanners against one captured Git snapshot
  -> require zero findings and exact inspected counts
```
