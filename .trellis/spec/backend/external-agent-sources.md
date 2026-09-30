# External Agent Product Sources and Desktop Identity

## 1. Scope / Trigger

Read this contract before changing product release discovery, redirect and
artifact admission, CLI package-source selection, desktop installation
identity, or the platform evidence used to turn a scanned application into a
trusted Agent candidate.

Primary owners are `src-tauri/src/agent_install/sources/**`, `fetch.rs`,
`desktop.rs`, `windows.rs`, `macos.rs`, and the product-specific source and
identity tables they compose. Shared Claude/Grok npm source mechanics are owned
here by `services/tooling/grok_npm.rs` plus the compact user-helper plan types.
[External Agent Lifecycle](./external-agent-lifecycle.md)
owns action legality, inventory normalization, opaque capabilities, jobs and
deployment orchestration. [Claude Code CLI](./claude-code-cli.md) owns
Claude-specific detection, owner/prefix admission and post-install
verification. Windows helper and Explorer-user execution boundaries are in
[Windows Agent Runtime Security](./windows-agent-runtime-security.md).

## 2. Signatures

```text
resolve_agent_source(agentId, surface, action)
  -> verified release metadata | source_not_verified

resolve_published_manifest(OfficialNpmTool::Grok | OfficialNpmTool::Claude)
  -> exact root version + root/platform SHA-512

fetch_release_capability(agentId)
  -> expectedReleaseId = "v1:" + 64 lowercase hex

enumerate_desktop_candidates(agentId, frozenUserContext)
  -> platform observations for inventory normalization

windows_exe_install_admitted(agentId)
  -> closed product identity is complete

uninstall_display_name_matches(displayName, closedNames)
  -> exact ASCII-case-insensitive name
     | name + " " + bounded stable version
```

The renderer never supplies a URL, raw path, registry, package, hash, signer,
Uninstall key, command or executable name. Source metadata and scanned paths
remain backend evidence and are projected only through lifecycle capabilities.

## 3. Contracts

### Product source matrix

| Product | Source and identity owner |
| --- | --- |
| Grok Build | CLI Tooling. Fresh install uses official `@xai-official/grok`; runtime resolves npm `latest` to one exact version plus root/current-platform integrity. Native `x.ai` install is explicit and updates preserve `native_internal` versus `official_npm`. |
| Claude Code | CLI only. This owner resolves official npm metadata and admitted mirrors; [Claude Code CLI](./claude-code-cli.md) owns discovery, owner-preserving update and verification. There is no public Claude Desktop source. |
| Codex | Dedicated Codex Desktop installer. Agent action returns `managed_by_codex_desktop`; this source owner does not duplicate it. |
| QoderWork CN | Reviewed first-party `/qoder-work-cn/releases/latest/` aliases plus same-host Electron-builder feed. Install/launch only; FyAgent update disabled. |
| TRAE Work CN | `data.solo` with `region=cn`; never TRAE Code or `data.manifest`. Comparable local version is bounded `tronBuildVersion`. |
| WorkBuddy | Closed `/v2/update` platform IDs and official download host. macOS rewrites only the validated terminal `.zip` suffix to `.dmg`. |
| OpenCode Desktop | Reviewed locale-neutral stable Desktop aliases and closed installed identity. Windows x64 uses `windows-x64-nsis`. Windows ARM64 has no stable alias: it reads the x64 alias redirect once without following it, accepts only `https://opencode.ai/files/bin/<x.y.z>/opencode-desktop-win-x64.exe` (no query, fragment, port or userinfo) and installs the same-version `opencode-desktop-win-arm64.exe`; that source is versioned (`versionless_latest=false`, version in the release ID). The aliases are versionless and may serve a different product line than GitHub Releases (since 2026-09 they serve v2 `2.0.19` while GitHub latest is `v1.18.33`), so GitHub latest is neither the display version nor the macOS expected release version. No public OpenCode CLI installer. |

### Official npm source boundary

- The signed product resolves a concrete version before composing npm argv.
  `@latest` and the command-shape `@xai-official/grok@1.2.3` fixture are never
  install authority.
- Root and current Darwin/Windows x64/arm64 optional-package SHA-512 values must
  match one allowed registry. Unsupported architecture produces no plan; Linux
  package support is not inferred from host-compilation tests. Published root
  `optionalDependencies` may list only the closed vendor suffix set for that
  product (`linux-x64`, `win32-x64`, `darwin-x64`, `linux-arm64`,
  `win32-arm64`, `darwin-arm64`; Claude also `linux-x64-musl` /
  `linux-arm64-musl`). Those Linux names are vendor metadata, not first-party
  Linux install support. A shared `{package}-` prefix is not enough. Every
  recognized sibling spec, including non-current vendor platforms, must pass
  the same bounded exact-version validator used for execution; `npm:` aliases,
  ranges, and tags are rejected. Distinct exact sibling versions may remain.
  Non-empty or malformed root `peerDependencies` are rejected. Fetched platform and
  `@iarna` child documents may omit `dependencies` / `optionalDependencies` /
  `peerDependencies` or use empty objects; any other child graph is
  `UnsupportedDependency`. Ordinary Grok root `dependencies` stay frozen to
  `@iarna/toml@3.0.0`.
- Version authority is npmjs `/latest` first, then the reviewed mainland
  metadata chain. Installation may use only a registry whose exact root and
  platform metadata match the resolved manifest.
- Only npmjs is the official version authority. A version selected by a mirror
  (`tencent` / `huawei` / `npmmirror`) is `mirror_fallback`: mirrors can lag or
  keep a stale `latest` dist-tag (observed 2026-09-30: npmmirror
  `@xai-official/grok` `latest` pointed at unpublished `0.1.4` while npmjs was
  `1.0.44`). The manifest and `ToolVersion.latest_authority`
  (`official` / `mirror_fallback`, omitted for non-npm sources) carry that
  state. The Grok panel titles a mirror fallback “镜像版本（官方最新未确认）” and the
  npm install confirmation shows `{version}（镜像版本，官方最新未确认）` instead of
  presenting it as official latest. The generic CLI readiness card does not yet
  carry `latest_authority` (follow-up). Exact-version argv and integrity
  checks are unchanged.
- Formal Windows receives only the compact exact-version, registry and
  allow-scripts control through the ordinary-user helper. Development Windows
  composes the same live plan in-process; npm 12+ receives only
  `--allow-scripts=@xai-official/grok`, and success requires PATH-default
  version readback. See the Windows Agent runtime contract for execution.
- macOS Claude/Grok discovery uses login-shell PATH, process PATH and product
  env. It does not walk mise/nvm/fnm/Volta internals. When several copies are
  visible, the PATH-default installation is the selected owner.
- CLI `ToolVersion.latest_version` remains live. Claude/Grok/Codex/Gemini/
  OpenClaw use npm metadata. Desktop products keep their vendor feeds. No
  reviewed CLI version/hash JSON is compiled into the product.
- Hermes latest is the official release's product version: fixed
  `NousResearch/hermes-agent` GitHub latest release (non-draft, non-prerelease,
  dotted three-component numeric tag, `html_url` bound to that repository) ->
  exact tag ->
  that tag's `pyproject.toml` `[project]` `name = "hermes-agent"` stable
  `version` (for example calendar tag `v2026.9.24` -> `0.21.5`, the value
  `hermes --version` reports). `latest_source = "official_release"`. PyPI is
  not an official install channel and lags (0.19.0 on 2026-09-30), the tag or
  release title is not the product version, and mutable `main` is never read;
  any failure is an unknown latest, never a PyPI fallback. Reads use the
  installer-proxy HTTPS-only, no-redirect, 20 s metadata client and are bounded
  to 1 MiB by declared length and streamed size.
- OpenCode publishes two official CLI lines from `anomalyco/opencode`:
  legacy `opencode-ai` (1.x, also the GitHub Releases line) and v2
  `@opencode/cli` (2.x). The observed local major selects the line for display
  only: major >= 2 reads `@opencode/cli` with no GitHub fallback; unknown,
  absent or 1.x reads `opencode-ai` with the GitHub latest fallback. No
  installation is migrated between lines.

### Desktop feeds and release capabilities

- Qoder display version is the unindented top-level `version:` from bounded
  same-host `latest.yml` / `latest-mac.yml`. Feed ZIP and `sha512` are metadata,
  not admitted artifacts. Windows ARM64 stays unsupported until separately
  reviewed first-party evidence exists.
- TRAE source selection uses the Work/Solo CN object and closed host/path/
  filename rules. Electron marketing `appVersion` is not the comparable
  runtime version.
- A shorter WorkBuddy dotted marketing version may equal a longer remote
  product-version prefix. Same-length differing segments remain an update.
- OpenCode Desktop resolution does not call GitHub at all; the stable alias
  stays installable and versionless. Windows ARM64 uses the versioned official
  file derived from the x64 alias redirect; an unparseable redirect fails closed
  with a source error, never a guessed URL.
- Every request is HTTPS, without userinfo or explicit non-default port. Each
  redirect hop must match the product allowlist and bounded hop count; scheme
  downgrade or an unknown host fails closed.
- Metadata is bounded to 1 MiB and artifacts to 2 GiB under the current
  transport. Cancellation maps to `cancelled`, not source/schema failure.
- `expectedReleaseId` binds the canonical backend-resolved release fields.
  A forced refresh must match before download. The capability never contains
  or exposes a URL.
- Missing/drifted schema, host, redirect or capability returns
  `source_not_verified` plus official-page guidance. Never pin an investigation
  URL or infer version from ETag, Last-Modified or prose.

### Closed desktop identity

Folder names and vendor configuration directories are not identity.

| Product | macOS bundle ID | Windows closed identity summary |
| --- | --- | --- |
| WorkBuddy | `com.tencent.workbuddy.mac` | Closed relative `WorkBuddy.exe`, ProductName and reviewed signer. |
| QoderWork CN | `com.qoder.work.cn` | Closed QoderWork CN relative EXE names, ProductName and signer. |
| TRAE Work CN | `cn.trae.solo.app` | Closed TRAE SOLO/Work CN relative EXE names, ProductName and signer. |
| OpenCode | `ai.opencode.desktop` | v2 `@opencodedesktop/OpenCode.exe`, 1.x `@opencode-aidesktop/OpenCode.exe` and installer-stub `OpenCode/OpenCode.exe`, ProductName `OpenCode`, reviewed signer `Anomaly Innovations, Inc https://anoma.ly/`, and exact/bounded-version Uninstall DisplayName. |
| Claude legacy identity | `com.anthropic.claudefordesktop` | Observation only; not an admitted Agent lifecycle target. Current Claude support is CLI-only. |

- Windows installed-target identity is distinct from the downloaded installer
  leaf. A trusted current-user OpenCode install may be AMD64 at
  `%LOCALAPPDATA%\\Programs\\@opencode-aidesktop\\OpenCode.exe` while the
  official NSIS stub is i386 at `OpenCode/OpenCode.exe`; preserve both closed
  relatives. The v2 installer (Electron package `@opencode/desktop`,
  `2.0.19`, same signer and ProductName, inspected offline on 2026-09-30 and not
  yet verified on a Windows host) derives the per-user directory
  `%LOCALAPPDATA%\\Programs\\@opencodedesktop\\OpenCode.exe`; it is admitted as a
  third closed relative.
- KnownPath `Missing` is dropped. Uninstall/App Paths entries remain hints and
  are inspected to the same closed PE identity before becoming candidates.
- Uninstall `DisplayName` accepts a closed name exactly or `{name}
  {bounded_stable_version}`. Reject channel words and prerelease suffixes.
  Empty `InstallLocation` is allowed; `DisplayIcon` and derived directories are
  evidence, never commands.
- OpenCode fresh Windows destination is `WindowsCurrentUser`. Its display label
  is not a scanned relative and must not generate one.
- macOS scans direct-child `.app` bundles in user/system Applications roots,
  rejects symlinks and verifies plist/bundle identity. Absence on a shipped
  host is `not_installed`; Linux development remains `unknown`.
- Windows uses the frozen Explorer user and explicit registry views. Inventory
  parents require query+enumerate; validated children are query-only. Optional
  absent/rejected shared-view links are absence, while access/enumeration/bound
  or Shell-context failure makes the aggregate unknown.
- A Windows candidate is actionable only after stable no-reparse file identity,
  supported architecture, closed ProductName, WinVerifyTrust, exactly one
  signer and the reviewed signer leaf.
- Do not infer installation from `.workbuddy`, `.qoderwork*`, `.trae*` or any
  settings directory.
- WorkBuddy macOS identity is `com.tencent.workbuddy.mac` across scan,
  system-commit policy and privileged helper. `com.workbuddy.workbuddy` is
  stale, and `com.tencent.codebuddycn` is another product.

### OpenCode Windows readback after vendor or manual NSIS

The same inventory scan must find an install launched by FyAgent or completed
manually. Helper product admission authorizes download/handoff only; it does
not prove scan relatives or installation completion.

`installState=installed` and `launch` require a trusted PE at a closed relative
or a matching Uninstall/App Paths hint that resolves to that identity. On an
ARM64 host the installer and installed EXE must be ARM64 PE (the i386 NSIS stub
remains installer-only). Successful `ShellExecute` is only vendor-wizard
handoff; status changes after a later complete scan.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Renderer supplies URL/path/registry/package/hash/signer/command | Reject before source or filesystem work. |
| npm `/latest` document yields tag text instead of concrete semver | Reject; no install plan. |
| Root or platform integrity differs at a candidate registry | Skip registry; source exhaustion if none match. |
| Development Windows uses `1.2.3` fixture or `@latest` | Contract regression; resolve exact live version. |
| npm 12+ omits narrow Grok allow-scripts or postinstall is blocked | Failure even when npm exits zero. |
| PATH-default CLI remains below planned version | Fail the attempt; do not report Agent success. |
| Desktop feed host/schema/redirect/port/body grammar drifts | `source_not_verified`; no stale pin. |
| Release capability changes during forced refresh | Reject before download/commit. |
| Closed desktop identity is incomplete | No install/launch claim; catalog cannot claim local recognition. |
| OpenCode `@opencode-aidesktop\\OpenCode.exe` passes closed identity | Candidate may become `installed` + `launch`. |
| DisplayName is `OpenCode 1.18.27` and points to the trusted EXE | Keep hint and inspect it. |
| DisplayName is `OpenCode Dev`, `OpenCodeAI` or prerelease | Skip. |
| KnownPath `OpenCode\\OpenCode.exe` is missing | Drop the observation; do not fail the aggregate. |
| Optional Windows parent is absent/rejected shared-view link | Absence; continue. |
| Windows parent cannot enumerate or frozen-user context is invalid | Aggregate `unknown`; no fresh destination. |
| Vendor wizard opens successfully | Handoff success only; no installed claim. |

## 5. Good / Base / Bad Cases

- **Good:** resolve one exact Grok npm manifest, verify root/platform integrity
  at an allowed registry, execute through the correct user boundary, then read
  back the PATH-default version.
- **Good:** a user runs the official OpenCode NSIS independently; the next
  complete scan finds `@opencode-aidesktop\\OpenCode.exe`, verifies PE identity
  and exposes Launch.
- **Base:** a vendor wizard opened but the later scan has not completed; the job
  records handoff while inventory remains unchanged.
- **Base:** a shipped-host source is unavailable; the UI receives official-page
  guidance without a cached URL or fabricated release.
- **Bad:** install `@latest`, compile reviewed CLI version JSON, trust a folder
  name/config directory, require exact `DisplayName == "OpenCode"`, or treat a
  destination label as a known path.

## 6. Tests Required

- Npm tests parse a concrete `/latest` version, reject `version=latest`, prove
  no version/hash JSON is included, match root plus current-platform integrity,
  and keep argv free of `@latest` and fixture `1.2.3` authority.
- PATH discovery tests prefer the PATH-default installation among several and
  do not walk manager internals.
- Windows LocalProcess tests add the Grok allow-scripts flag only for npm 12+,
  preserve an existing flag, reject blocked-install-script output and require
  final PATH-default version readback.
- Source parser tests enforce exact host, platform, schema, redirects, body
  bounds and release capability refresh for Qoder, TRAE, WorkBuddy and OpenCode.
- Desktop identity tests freeze bundle IDs, relative EXEs, ProductName, signer,
  architecture and no-reparse file identity.
- OpenCode tests keep both installed-target and stub relatives, accept exact or
  bounded stable-version DisplayName, reject channel/prerelease lookalikes,
  drop missing KnownPath and expose `WindowsCurrentUser` only after complete
  absence.
- Matching-host Windows/macOS tests remain required for native trust behavior;
  portable fixtures do not prove WinVerifyTrust, registry views, signer or
  package execution.

## 7. Wrong vs Correct

### Wrong

```text
npm i -g @xai-official/grok@latest
npm exit 0 -> installed
```

```rust
windows_relative_exes: &["OpenCode/OpenCode.exe"];
if display_name != "OpenCode" { continue; }
```

```text
Hermes latest = PyPI hermes-agent            # lagging, not an official channel
OpenCode Desktop display = GitHub latest     # alias serves another line
npmmirror latest 0.1.4 -> "official latest"  # mirror dist-tag is not authority
```

### Correct

```text
resolve live exact manifest
  -> match root/current-platform integrity
  -> closed user execution
  -> PATH-default version readback
```

```rust
windows_relative_exes: &[
    "@opencodedesktop/OpenCode.exe",
    "@opencode-aidesktop/OpenCode.exe",
    "OpenCode/OpenCode.exe",
];
uninstall_display_name_matches(display_name, &["OpenCode"])
// exact name, or `OpenCode` + space + bounded stable version
```
