# Platform-sensitive source review

Integration baseline: `2c09c4be`; final upstream `da91427e` is fetched and awaits
merge after active writers stop. This is a review record, not a passing check.

## Reviewed dispatch and source changes

- Installer inventory caches one native-prepared target and consumes it once.
  Preflight reruns at apply and compares request, native paths, runtime and
  execution context. No renderer path becomes authority.
- New installer preflight uses explicit macOS and Windows paths. macOS checks
  writable user destinations and admits system authorization only through the
  existing production capability. Windows keeps the desktop-user context and
  vendor wizard as authority. Unsupported hosts return PlatformUnsupported.
- CLI preflight uses the selected existing installation on macOS and the
  existing authenticated interactive-user helper on Windows. Closed helper
  wire codes 18–21 add read-only preflight, without package admission.
- Codex macOS filesystem writability uses access only under target_os=macos;
  the non-macOS implementation rejects. Prepared install packages now bind the
  confirmation target; platform dispatch is unchanged.
- New config-pack file owner rejects symlink/reparse ancestors, bounds reads
  and validates structure before returning text; export is create-only. Its
  no-follow read flags have explicit supported-host guards. The symlink test
  is scoped to macOS rather than a broad host family.
- Shared file restore scope rechecks the exact expected postimage under WRITER,
  rejects unplanned files, and updates expected hashes for internal writes.
  It remains synchronous and cannot cross await.
- WorkBuddy removed indiscriminate old-backup restore. Its macOS primary writer
  alone compensates a publication against the operation's preimage and exact
  replacement hash. Failure before publication and later external edits stay
  untouched. Windows retains its existing storage transaction.
- Secret Memory backend is available for the explicit Database::memory fixture
  constructor. Persistent Database initialization still chooses native secret
  storage; there is no native-failure fallback to Memory.
- Native startup registers configuration-pack/preflight commands and performs
  deferred-safe legacy credential migration. It never writes external Agent
  config during migration.
- Codex catalog helper visibility changes enable owned proxy restoration; no
  platform dispatch changed. Protocol probes retain native network validation.
- Auth Grok unsupported projection reports unknown and does not claim a native
  subscription or preserved official session without observed evidence.

## Scanner guard change

Four narrowly named new fallback allowances include the entire normalized
platform attribute and rejecting body. Negative tests replace Err with Ok or
false with true and require both implicit-target and allowance-drift findings.
These 23 scanner tests pass; two inventory tests remain intentionally failing
until final source bytes are reviewed and the manifest is updated. Existing
allowances and all source/text scanners remain enforced.

## Current reviewed source identities

The 26 reviewed source changes are now recorded, with 119 total candidate paths
and no removals. The storage rejection allowance now includes the actual
`required_bytes` argument and retains its entire rejecting body; it does not
broaden the allowed platform condition. Two unused Codex route-stripping helpers
and their sole tests were removed after owned restoration replaced them; source
search confirms no remaining caller. Common-overlay credential material is
validated before native use, with no second overlay applied after resolution.

The final upstream merge and the separate CLI work package may change source
identities again. Review their actual final differences before updating the
corresponding entries. The current manifest is an integration checkpoint, not a
claim that all final checks have passed.

Installer free-space admission beyond zero is also under focused review; do not
claim adequate space merely from a nonzero available-byte observation.

## Post-main common projection review

Main `da91427e` is merged in `35848bef`. The subsequent candidate scan still has 119 paths, with only `src-tauri/src/codex_config.rs` and `src-tauri/src/services/provider/mod.rs` changed. GPT-6 reviewed both actual deltas: the first replaces the unused plain writer wrapper with the shared in-memory common-snippet projection and one atomic write; the second gives preview that same explicitly enabled snippet. Neither changes supported-host admission, platform attributes, package targets or native dispatch. Their exact source digests are updated after this review. No inventory additions, removals or fallback allowances. The still-separate CLI delta must receive its own source review.

## Adopted demo raster identities

The scanner reports exactly seven new raster candidates, all under
`docs/fyagent/development/demos/0.4.6-fac051ea/`. Root already visually reviewed
all seven actual screenshots while adopting the demo: first-use purpose and
recommendations, existing configuration, WorkBuddy setup, save preview, recovered
failure and saved result. They describe the actual fixture UI, show no extra
supported operating system or release claim, and contain no real credentials or
concrete personal home path. Their current SHA-256 values were matched against
the immutable capture manifest before adding these exact seven entries to the
raster inventory. All existing entries and hashes remain unchanged. This is
asset review, not native or release acceptance.

## Closed vendor npm metadata tables

The final CLI dependency admission requires six Grok and eight Claude vendor package suffixes, including foreign-platform sibling identities. Those names describe registry data, not product dispatch. Root added a narrow scanner contract admitting only the complete two exact suffix tables in their sole host metadata owner. All surrounding code and other scanner rules still run. Negative fixtures reject changed/missing values, duplicate tables, moved owners, appended content, and a new foreign-host selector. The focused scanner fixture passed; complete repository validation remains part of the final gate.

Final identities were recomputed only for the reviewed CLI target/plan/codec paths, helper dependency manifest, new bounded dependency-reader module, and corresponding scanner/contract tests. The unsupported-host preflight source contract was updated for its new optional plan parameter while retaining the exact `PlatformUnsupported` body. The two adopted videos and two byte-identical raw copies extend the existing reviewed media inventory; finite container bounds and non-frame metadata checks were added without excluding video paths from identity or filename scanning.

Final test-only identity refresh: projects/tests.rs changes only the read-only snapshot assertion, excluding the volatile export header and retaining SQL content plus total-change checks. Existing host gates are unchanged.
