# Acceptance evidence ledger

This ledger is initialized during planning. Implementation agents append concrete evidence; design assumptions do not close gates.

## Status legend

- `VERIFIED_LOCAL`: directly observed on the test Mac/repository.
- `VERIFIED_SOURCE`: confirmed in primary vendor/platform source.
- `AUTOMATED`: passing automated test with command/commit.
- `HIL`: reproduced on real hardware/signed build.
- `OPEN`: not yet proven.
- `BLOCKED`: evidence proves the planned route unavailable.

## Planning evidence

| ID | Status | Evidence | Date |
| --- | --- | --- | --- |
| P-01 | VERIFIED_LOCAL | `/Applications/OpenCode.app` exists; Bundle ID `ai.opencode.desktop`; version observed `1.18.19`; LaunchServices/Spotlight resolves it. | 2026-08-31 |
| P-02 | VERIFIED_LOCAL | Grok command uses vendor `~/.grok` internal layout; version observed `1.0.5`; config owner `internal`, channel `stable`. | 2026-08-31 |
| P-03 | VERIFIED_LOCAL | Managed desktop registry omits OpenCode; OpenCode is modeled as CLI only. | 2026-08-31 |
| P-04 | VERIFIED_LOCAL | Generic Agent macOS path buffers full DMG and writes a second complete temp artifact. | 2026-08-31 |
| P-05 | VERIFIED_LOCAL | Codex equal-or-newer install branch calls launch. Exact red warning remains unknown because its text/log was not captured. | 2026-08-31 |
| P-06 | VERIFIED_LOCAL | Existing scanner already covers direct children of `/Applications` and `~/Applications`; the verified OpenCode miss does not require a global scanner. | 2026-08-31 |
| P-07 | VERIFIED_SOURCE | Apple documents privileged-file-operations authorization; signed feasibility is still required for fresh create/exact transaction. | 2026-08-31 |
| P-08 | VERIFIED_SOURCE | xAI native distribution owns x.ai primary, official GCS fallback, channel/arch/layout validation; official npm is a separate supported distribution. | 2026-08-31 |
| P-09 | VERIFIED_SOURCE | OpenCode publishes separate desktop DMGs for Apple Silicon and Intel. | 2026-08-31 |
| P-10 | VERIFIED_SOURCE | Blessed/SecureXPC provide reviewed helper lifecycle/XPC primitives; Mist demonstrates a macOS 12+ production integration. | 2026-08-31 |

## G1 — Reuse and owner convergence

Status: `OPEN`

Required evidence:

- [x] Codex and managed desktop products call one artifact transport core. (`AUTOMATED` Wave 1 A: Agent `fetch.rs` delegates `prepare_transport_download` + `persist_transport_response`; no second downloader. G1 still `OPEN` for plist/launch/negative-scan remainder.)
- [ ] One managed DMG transaction.
- [x] One structured bundle metadata owner. (`AUTOMATED` Wave 2 D: Codex `parse_structured_info_plist_json` / `read_structured_bundle_plist`; managed discovery no longer owns XML `<key>CFBundle` scanning)
- [x] One managed desktop inventory. (`AUTOMATED` Wave 2 D: OpenCode added to existing `DESKTOP_PRODUCTS`; no Launch Services scanner)
- [x] One process-launch business owner with native macOS adapter. (`AUTOMATED` Wave 2 D: managed `launch_macos_bundle` calls `launch_trusted_macos_application_as_user`; no second launcher)
- [x] One frontend transfer projector. (`AUTOMATED` Wave 2 E: `src/shared/codex-desktop/snapshots.ts` `projectTransferPresentation` / `formatTransferPercent`; Codex panel, leftover card, Agent job hook, and directory busy copy consume it. Tests: `tests/shared/codexDesktopCore.test.ts` shared transfer projector; `tests/v2/shared/features/transfer-projection.test.ts`.)
- [ ] Old duplicate code removed; negative scans attached.

### G1 authorization spike (Wave 1 / R — Apple `/Applications`)

Wave-ownership G1 for native `/Applications` authorization. Distinct from the reuse checkboxes above. PRD G2 remains the later adapter-selection gate.

Spike status: `OPEN` (not VERIFIED). Native signed HIL: `BLOCKED`. Do not enable system commit.

Detail: `research/g1-authorization-spike.md`.

| ID | Status | Evidence | Date |
| --- | --- | --- | --- |
| G1-AUTH-01 | VERIFIED_LOCAL | `src-tauri/entitlements.macos.plist` contains only `com.apple.security.cs.allow-jit`, `allow-unsigned-executable-memory`, `disable-library-validation`. `com.apple.developer.security.privileged-file-operations` is absent. | 2026-08-31 |
| G1-AUTH-02 | VERIFIED_LOCAL | Formal `scripts/release/macos-developer-id.sh sign-app` uses `--entitlements` on that plist with hardened runtime + timestamp. No `embedded.provisionprofile` copy. `verify-macos-signed-app.sh` does not check entitlement keys or a profile. | 2026-08-31 |
| G1-AUTH-03 | VERIFIED_LOCAL | Installed `/Applications/FyAgent.app` 0.4.2 is Developer ID `HY446996QX`, runtime flagged, notarized path; entitlements match the plist; `Contents/embedded.provisionprofile` missing. Current unrestricted entitlements survive notarization; the restricted key is not in the package. | 2026-08-31 |
| G1-AUTH-04 | VERIFIED_SOURCE | Apple entitlement abstract: create symbolic links, replace files, set attributes. Request form required. Not listed on Supported capabilities (macOS). Restricted entitlements require a provisioning profile (TN3125). Forums/DTS: generally Mac App Store; Developer ID enablement is manual DTS, not a self-serve Additional Entitlements checkbox. | 2026-08-31 |
| G1-AUTH-05 | VERIFIED_SOURCE | MacOSX.sdk 26.5 `NSWorkspace.h`: `NSWorkspaceAuthorizationType` is CreateSymbolicLink / SetAttributes / ReplaceFile only, `API_AVAILABLE(macos(10.14))`. Authorized FileManager methods are exactly those three; other FileManager methods “behave normally”. `replaceItemAtURL` replaces an existing original item. Fresh absent `/Applications/<App>.app` copy/create is outside the authorized set. | 2026-08-31 |
| G1-AUTH-06 | VERIFIED_LOCAL | `objc2-app-kit` 0.2.2 (direct) and 0.3.2 (transitive) expose the same three types plus `requestAuthorizationOfType_completionHandler` / `fileManagerWithAuthorization`. Production crate enables `NSColor` only; 0.2 needs features `NSWorkspace` + `block2` to compile the authorization API. Bindings do not add copy/create. | 2026-08-31 |
| G1-AUTH-07 | BLOCKED | Signed Developer ID HIL cannot run in this environment: `security find-identity -v -p codesigning` → 0 identities; `FYAGENT_APPLE_*` unset; no repo profile; Apple secrets exist only on formal GitHub `build-macos`. Unsigned debug is not HIL. Host is macOS 26.6.2, not macOS 12. | 2026-08-31 |
| G1-AUTH-08 | VERIFIED_LOCAL | `agent_install` fresh system and all-users update return `AuthorizationRequired`. No sudo, AppleScript admin, `AuthorizationExecuteWithPrivileges`, generic helper, or silent `~/Applications` success path in that executor. | 2026-08-31 |
| G1-AUTH-09 | OPEN | Wave 2 keeps `/Applications` one-click disabled/manual until a later signed/notarized HIL (and, for native Gate A, operations the SDK currently excludes for fresh create). Gate B helper not in this spike. | 2026-08-31 |

G2 native-authorization checkboxes below stay unchecked: entitlement is not in the Developer ID package, and signed fresh-create / replace / rollback / cancel / macOS 12 HIL were not run.

## G2 — System-commit adapter selection

Status: `OPEN`

### Native authorization evidence

- [ ] entitlement approved/preserved in actual Developer ID package.
- [ ] signed fresh absent-target commit.
- [ ] signed exact replacement and rollback.
- [ ] cancel/deny/expired authorization keeps target/staging known.
- [ ] macOS 12/current HIL.

### Conditional helper evidence

Only required if native is insufficient:

- [ ] reason native cannot satisfy contract documented.
- [ ] Blessed/SecureXPC license/version/maintenance/supply-chain review.
- [ ] helper embedded, blessed, signed and notarized.
- [ ] mutual signer, version/downgrade, replay and containment tests.
- [ ] closed operation protocol with no arbitrary path/URL/command.
- [ ] macOS 12/current HIL.

Selection:

- [ ] exactly one production adapter retained.
- [x] if blocked, system action remains disabled/manual with no user-scope fallback.

Wave 2 lock (2026-08-31): **helper / system `/Applications` writes are out of this task** (user). Native Gate A remains insufficient. Keep `MacSystemApplications` disabled/`authorization_required`. Do not implement Gate B here.

## G3 — OpenCode official source and desktop surface

Status: `OPEN` (Wave 2 D: source/classifier/plist/surface contract `AUTOMATED`; live `/Applications` discovery remains planning `P-01`; explicit launch HIL and signed package remain `OPEN`)

Wave 2 D (2026-08-31): OpenCode Desktop is in the existing managed registry (`ai.opencode.desktop`). Official DMGs are versionless stable aliases (`darwin-aarch64-dmg` / `darwin-x64-dmg`); researched version `1.18.19` is not a constant. Discovery uses Codex bounded `plutil -> JSON -> typed fields`. Hand-written XML plist scan and `Command::new("open")` are gone from `agent_install/desktop.rs`. Launch delegates to `process_launch`. Fresh install stays `~/Applications`. System writes remain deferred.

- [x] official release metadata/identity strategy recorded. (`AUTOMATED` `sources/opencode.rs`: opaque `v1:` release id from product/surface/platform/arch/stable alias; `display_version` unset; no research-time version literal)
- [x] arm64 DMG unique classifier test. (`AUTOMATED` `opencode_desktop_maps_each_macos_arch_to_exactly_one_official_dmg`)
- [x] x64 DMG unique classifier test. (`AUTOMATED` same)
- [x] ambiguous/missing asset fails closed. (`AUTOMATED` Windows/off-arch `PlatformUnsupported`; cross-arch path tokens `ArtifactRejected`; ghproxy host rejected)
- [x] shared structured reader verifies installed Bundle ID `ai.opencode.desktop`. (`AUTOMATED` binary+XML plist discovery; wrong Bundle ID / symlink / nested / corrupt plist not trusted)
- [ ] current `/Applications/OpenCode.app` detected by existing inventory. (planning `P-01` VERIFIED_LOCAL on the test Mac; CI uses temp known-roots, not a signed HIL scan of the live path)
- [x] CLI-only/Desktop-only/both/neither tests. (`AUTOMATED` TS parser four combos; Rust wire independent `cli`/`desktop` surfaces; CLI `launch` rejected)
- [ ] explicit “打开软件” HIL.

## G4 — Grok owner-aware official behavior

Status: `OPEN` (Wave 1 owner B: macOS command/owner/job unit evidence `AUTOMATED`; live installer, GCS, mainland, and signed HIL remain `OPEN`)

Wave 1 B (2026-08-31): Tooling keeps both `native_internal` and `official_npm`. macOS no longer composes `installer || npm` or `grok update || installer`. Native latest uses `grok update --check`; npm latest stays on the official package registry. Persistent terminal snapshots live in Tooling (`last_grok_lifecycle_snapshot`); Agent UI / `install_official_npm` wiring is Wave 2. Windows `|| npm` install tests are unchanged.

### Native/internal

- [x] anchored `grok update --check`/frozen update works. (`AUTOMATED` command composition + `--check` parser; live `--version` HIL `OPEN`)
- [ ] fresh official installer works.
- [ ] x.ai primary success.
- [ ] x.ai failure -> official GCS fallback success.
- [ ] both unavailable -> persistent terminal failure.
- [x] exit/timeout/cancel/output redaction tested. (`AUTOMATED` redaction + timeout reason mapping; user-cancel path `OPEN`)
- [x] failed update preserves prior binary/symlink/internal owner. (`AUTOMATED` plan: native failure does not invoke npm; live binary preservation HIL `OPEN`)

### Official npm

- [ ] explicit fresh install through configured registry.
- [x] npm-owned update stays npm-owned. (`AUTOMATED` owner-bound plan)
- [x] native failure never automatically invokes npm. (`AUTOMATED`)
- [x] user-explicit owner switch, if offered, is a new action/job. (`AUTOMATED` closed action `install_official_npm`; Wave 2 E Agent/Settings picker calls existing `run_tool_lifecycle_action` only on click; native failure does not auto-invoke)

### Network

- [ ] proxy behavior recorded.
- [ ] mainland-network native HIL.
- [ ] mainland-network official npm HIL.
- [x] no unreviewed mirror. (`AUTOMATED` installer URL allowlist rejects non-`x.ai` hosts, including GCS script fetch)

## G5 — Transfer/job UX

Status: `OPEN` (Wave 1 A wire `AUTOMATED`; Wave 2 E projector/percent/speed/CLI installer copy `AUTOMATED`; signed HIL and some terminal-persistence cases remain `OPEN`)

- [x] generic job has monotonic sequence/timestamps and raw transfer bytes. (`AUTOMATED` action contract v3; `record_transfer` monotonic per attempt)
- [x] one-decimal percent. (`AUTOMATED` Wave 2 E: `formatTransferPercent(37.44) === "37.4%"`; Agent hook `progressLabel` `下载中 37.4%`; clamp `0..100`)
- [x] true speed shown only when fresh byte samples exist. (`AUTOMATED` `selectDownloadBytesPerSecondFromSample` hides non-downloading / ≤0 / sequence mismatch; terminal presentation `speedLabel` null; no `0 B/s`)
- [x] unknown total remains indeterminate. (`AUTOMATED` wire `totalBytes` null; UI `已下载 126 MB` without invented percent)
- [x] external installer without bytes does not fake progress. (`AUTOMATED` Wave 2 E: Grok official npm panel shows stage copy only, no percent/speed; CLI stages without `transfer` keep `percent: null`)
- [ ] failed/cancelled/rollback/recovery terminal states persist.

## G6 — Signed product HIL

Status: `OPEN`

- [ ] Apple Silicon native.
- [ ] Intel device/trusted runner.
- [ ] macOS 12 and current supported macOS.
- [ ] five desktop products fresh `/Applications` install.
- [ ] system/user exact-location update.
- [ ] rollback, cancellation, target drift, multiple candidates.
- [ ] Codex no implicit launch.
- [ ] OpenCode detection/version/explicit launch.
- [ ] transfer progress/speed/unknown length.

## Automated evidence template

| Commit | Command/test | Scope | Result | Notes |
| --- | --- | --- | --- | --- |
| `<pending>` | `<pending>` | `<pending>` | `<pending>` | `<pending>` |
| uncommitted Wave 1 A | `mise run rust:test -- transfer` | G5 job transfer snapshot | pass (3 tests) | Contract v3 closed `transfer`; terminal keeps bytes, ignores later samples |
| uncommitted Wave 1 A | `mise run rust:test -- agent_install::fetch` | G5 streaming DMG persist | pass (4 tests in lib) | Streams to `installer.dmg`; unknown Content-Length does not invent total; EXE source rejected |
| uncommitted Wave 1 A | `mise run rust:test -- downloads_to_a_fixed_local_name` | Codex persist no-regression | pass | Agent delegates this owner; no second downloader |
| uncommitted Wave 1 A | `mise run typecheck:v2` + `mise run test:v2 -- tests/v2/features/agent-install-readiness.test.ts` | G5 TS job parser v3 | pass (5 tests) | Rejects v2, `percent`, `path`, `totalBytes < completedBytes` |
| uncommitted Wave 1 B | `cargo test --manifest-path src-tauri/Cargo.toml --features fyagent/test-hooks --lib services::tooling` | G4 macOS Grok owner/plan/redaction | pass (100 tests) | No live x.ai/GCS/mainland HIL. Windows `\|\| npm` composition not changed. |
| uncommitted Wave 1 C | `cargo test --manifest-path src-tauri/Cargo.toml --lib -- platform::process_launch services::codex_desktop codex_desktop::jobs` | explicit launch / no auto-launch | pass (61 tests in this filter) | Codex equal-or-newer is `AlreadyCurrent` readback; install success does not launch; restart/explicit launch still launch. macOS adapter uses NSWorkspace completion. No signed HIL. |
| uncommitted Wave 2 E | `mise run typecheck:v2` | G5 UI types | pass | Consumes Wave 2 D `surfaces[]`; no second readiness parser |
| uncommitted Wave 2 E | `mise run test:v2 -- tests/shared/codexDesktopCore.test.ts tests/v2/shared/features/transfer-projection.test.ts tests/v2/pages/agents/useAgentLifecycleAction.test.tsx tests/v2/pages/agents/AgentInstallReadinessSection.test.tsx tests/v2/pages/agents/CodexDesktopInstallerPanel.test.tsx tests/v2/pages/agents/Page.test.tsx tests/v2/features/grok-tooling.test.ts tests/v2/platform/grokToolingPort.test.ts` | G5 UX + 7.4/7.5 UI | pass (59) | Shared projector; exact `打开软件`; OpenCode four surface combos; Grok explicit npm / native-fail switch; system dest stays disabled |

## Wave 2 E — Agent page UX (progress, 打开软件, OpenCode dual surface, Grok picker)

Status: `AUTOMATED` for frontend projector and page copy. Helper / `/Applications` writes remain deferred. Wave 2 D surface wire still independently owned.

| ID | Status | Evidence | Date |
| --- | --- | --- | --- |
| W2E-01 | AUTOMATED | One transfer projector in `snapshots.ts` (`formatTransferPercent`, `formatTransferSpeed`, `projectTransferPresentation`). Codex V2 panel, leftover Codex card, Agent `useAgentLifecycleAction`, and directory busy copy consume it. 37.44 → `37.4%`. Unknown total → indeterminate + transferred bytes. Terminal and ≤0 speed hidden. No page-local `toFixed(1)` progress. | 2026-08-31 |
| W2E-02 | AUTOMATED | Desktop launch copy is exactly `打开软件` (Agent section + Codex V2 panel). CLI surface does not show it. Update success does not call `startAction({ action: "launch" })`. | 2026-08-31 |
| W2E-03 | AUTOMATED | OpenCode configuration shows 命令行 + 桌面应用 in one 安装与更新 region. After Wave 2 D, UI consumes `readiness.surfaces[]` as authority: top-level is CLI only; desktop install/update/launch send `surface: "desktop"`; `getInventory("opencode", "cli" \| "desktop")`. Four combos (neither / CLI-only / Desktop-only / both) tested. Compact products stay one section. | 2026-08-31 |
| W2E-04 | AUTOMATED | Grok owner/latest labels stay owner-scoped. Agent page shows `使用官方 npm 方式` on first install and `改用官方 npm 方式` only after native failure; click calls `installOfficialNpm`. Native failure does not auto-switch. Official npm path shows stage copy, not fake bytes. CLI never shows `打开软件`. | 2026-08-31 |
| W2E-05 | AUTOMATED | System `/Applications` remains unavailable copy (`authorization_required` → 不可用于一键安装 / 不会改装到其他目录). Disabled system destination stays visible in the picker and is not one-click. No helper/XPC UI. | 2026-08-31 |

## Wave 1 A — Shared artifact transport + transfer telemetry

Status: `AUTOMATED` for streaming persist + job `transfer` wire. Frontend percent/speed projector remains Wave 2. G1 overall still `OPEN`.

| ID | Status | Evidence | Date |
| --- | --- | --- | --- |
| W1A-01 | AUTOMATED | Deleted production `fetch_artifact_bytes` / `download_macos_dmg_bytes` `Vec<u8>` path. macOS Agent download is `download_macos_dmg_to_job` in `fetch.rs` → Codex `prepare_transport_download` + `persist_transport_response` → job-local `installer.dmg`. `deploy_macos_dmg` revalidates and passes `artifact.path()` to `install_managed_exact` (no second full DMG write). Tests: `macos_dmg_streams_to_the_fixed_job_local_artifact`, `unknown_content_length_still_persists_without_inventing_a_total`, `macos_dmg_download_rejects_windows_exe_sources`. `collect_body` remains metadata-only. | 2026-08-31 |
| W1A-02 | AUTOMATED | Agent action job contract bumped to v3 with closed `transfer` (`phase`, `completedBytes`, optional `totalBytes`, `attempt`, `maxAttempts`, `sequence`, `observedAt`). No `percent` / `bytesPerSecond` / path / URL on the wire. Tests: `action_job_snapshot_carries_closed_transfer_telemetry`, `transfer_samples_are_monotonic_per_attempt_and_reset_on_retry`, `malformed_or_pre_download_samples_do_not_create_transfer`. TS `parseAgentActionJobSnapshot` rejects v2 and extra locator fields. | 2026-08-31 |
| W1A-03 | AUTOMATED | Windows EXE still streams via `fetch_artifact_to_job` / `ArtifactKind::Exe` with the same persist owner and a progress sink. Retry remains single-attempt so Windows behavior is unchanged. `/Applications` commit, OpenCode registry, `AgentSurface`, launch, and Grok were not modified in this wave. | 2026-08-31 |

## Wave 1 C — Explicit launch / no implicit launch

Status: `AUTOMATED` for service + `process_launch` unit tests. Signed NSWorkspace HIL remains `OPEN`.

| ID | Status | Evidence | Date |
| --- | --- | --- | --- |
| W1C-01 | AUTOMATED | Equal-or-newer Codex install no longer calls `platform.launch`. Job settles with `succeed_already_current` from Checking. Tests: `direct_install_request_treats_an_equal_local_version_as_already_current_without_launching`, `direct_install_request_keeps_a_newer_local_version_without_downgrading_or_launching`. | 2026-08-31 |
| W1C-02 | AUTOMATED | Install happy path does not launch. Explicit `CodexDesktopService::launch` and restart still launch. Tests: `happy_path_revalidates_downloads_verifies_installs_and_cleans_up`, `only_the_checked_release_id_can_claim_a_job_slot_and_launch_stays_local`, restart confirmation tests. | 2026-08-31 |
| W1C-03 | AUTOMATED | `process_launch` rejects relative / NUL / `..` / non-`.app` before any adapter. Production macOS path uses `NSWorkspace.openApplicationAtURL:configuration:completionHandler:`. Completion errors map to `external_launch_failed` with domain category logs (`cocoa` / `posix` / `os_status` / `workspace` / `other` / `timeout`), never a user path. | 2026-08-31 |

## HIL evidence template

| Date | Build/signing identity | macOS/hardware | Network class | Product/action | Expected | Observed | Redacted artifact | Result |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-08-31 | none (0 local Developer ID identities; `FYAGENT_APPLE_*` unset; no `embedded.provisionprofile`) | macOS 26.6.2 / this Mac | n/a | G1 native `NSWorkspace.requestAuthorization` + privileged-file-operations | signed/notarized HIL of entitlement, fresh `/Applications` create, replace, rollback, cancel | not executed; unsigned debug not used as proof | `research/g1-authorization-spike.md` | BLOCKED |

Do not record tokens, full home paths, serial numbers, public IPs or unredacted installer logs.
