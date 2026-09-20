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
