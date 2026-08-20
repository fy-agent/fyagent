# Research: Codex one-click installer verification

- **Query**: Remaining hash/checksum/signature/size/identity admission checks in Codex desktop/CLI one-click install; distinguish upstream publication-field admission vs local job integrity / OS installer trust / post-install existence.
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

User requirement (item 1): remove verification during Codex one-click install because maintaining checksums/signatures against changing upstream packages is costly, future one-click installers would inherit that cost, and verification makes install slow.

Current spec `.trellis/spec/backend/codex-desktop-installer.md` already forbids class (a) upstream publication-field admission and keeps class (b) local handoff / OS trust / post-install existence. Code matches that split: (a) is gone from the download/admit path; (b) still hashes the whole local artifact multiple times and still runs `verifying_installation`.

### Classification used below

| Class | Meaning | User item 1 | Current spec |
|---|---|---|---|
| (a) Upstream publication-field admission | Compare downloaded bytes or native package contents to mirror/manifest SHA, Content-Length, identity, publisher, Team ID, version, architecture, signature, notarization, Gatekeeper | Remove | Already forbidden |
| (b) Local job integrity / OS installer trust / post-install existence | Local same-file size/hash across job/privilege boundaries; native OS installer; post-install one operational result | User described as “校验” (slow) | Spec currently keeps |

### Files Found

| File Path | Description |
|---|---|
| `.trellis/spec/backend/codex-desktop-installer.md` | Contract: no upstream hash/size/identity/signature admission; keep local fingerprint, OS installer, post-install existence |
| `src-tauri/src/codex_desktop/source.rs` | Manifest parser ignores `sha256` / signature / publisher; keeps `content_length` only as `download_size_hint` |
| `src-tauri/src/codex_desktop/download.rs` | Streams SHA-256 of received bytes; does not compare to remote checksum; `Content-Length` mismatch does not fail |
| `src-tauri/src/codex_desktop/verify.rs` | Local fingerprint + `verify_reader` against locally computed size/hash; disk-space preflight |
| `src-tauri/src/codex_desktop/platform.rs` | `WINDOWS_CODEX_STABLE_IDENTITY` / `MACOS_CODEX_STABLE_IDENTITY`; `PreparedInstallPackage::revalidate_artifact` |
| `src-tauri/src/codex_desktop/platform/windows/helper.rs` | Full-file `verify_reader` at pin open and recheck |
| `src-tauri/src/codex_desktop/platform/windows/package_bridge.rs` | Copy-time SHA-256 vs locally expected digest |
| `src-tauri/src/codex_desktop/platform/windows/mod.rs` | Install-time `revalidate_artifact`; post-install inventory delta; Stable identity used for discovery |
| `src-tauri/src/codex_desktop/platform/macos/dmg.rs` | Install-time `revalidate_artifact`; no remote Team ID / codesign admission |
| `src-tauri/src/codex_desktop/platform/macos/bundle.rs` | Stable bundle ID for already-installed discovery, not downloaded-content admission |
| `src-tauri/src/services/codex_desktop/mod.rs` | Job stages including `VerifyingInstallation`; disk preflight from size hint |
| `src/v2/shared/codex-desktop/CodexDesktopInstallerPanel.tsx` | UI copy for `job_verifying_installation` |
| `src-tauri/src/commands/misc.rs` | Generic CLI lifecycle; Codex CLI install/update is not writable |
| `src/i18n/locales/zh.json` | `checksumMismatch` / `verifyingInstallation` copy |

## Code Patterns

### One-click surface that still exists

Codex one-click executable install is **Codex Desktop** (V2 Agent catalog mounts `CodexDesktopInstallerPanel`; V1 mounts `CodexDesktopInstallerCard`). IPC start input is only `expectedReleaseId` (`codex-desktop-installer.md` §2).

Codex **CLI** is not a one-click installer in this repo:

```126:130:src-tauri/src/commands/misc.rs
/// 生命周期写权限比只读探测更窄。Codex 保留在 `VALID_TOOLS`，因此版本和安装分布
/// 诊断仍可用；但不得规划或执行 install/update/repair 命令。
fn is_lifecycle_writable(tool: &str) -> bool {
    tool != "codex"
}
```

`npm_install_command_for("codex")` is `None`. There is no Codex CLI hash/signature admission path to remove. Other tools (claude/opencode/hermes/grok) install via official shell scripts or npm with no FyAgent checksum gate.

### (a) Upstream publication-field admission — not present in current download admit path

Manifest structs do not deserialize `sha256`, `packageMoniker`, `signature`, `publisher`, `minimumOsVersion`, or remote URLs. Windows/macOS artifacts keep `content_length` only as a hint:

```609:631:src-tauri/src/codex_desktop/source.rs
struct RawWindowsArtifact {
    status: Option<String>,
    downloadable: Option<bool>,
    version: Option<String>,
    content_length: Option<u64>,
}
// ...
struct RawMacosArtifact {
    content_length: Option<u64>,
    bundle_short_version: Option<String>,
    bundle_version: Option<String>,
    downloadable: Option<bool>,
    status: Option<String>,
}
```

Unknown publication fields are ignored by serde. Test `manifest_only_resolution_ignores_upstream_content_admission_fields` (`source.rs:56-69`) mutates `sha256` / `packageMoniker` / `minimumOsVersion` / `signature` / `contentLength` and still resolves; `contentLength` becomes `download_size_hint`.

Download computes a **local** SHA-256 while writing; it does not compare to a manifest digest. `Content-Length` / `download_size_hint` are progress totals only (`download.rs:595-599`). Empty bodies and the 8 GiB `MAX_ARTIFACT_BYTES` cap still fail. Test `remote_checksum_drift_does_not_block_the_download` (`download.rs:1342`).

Service test `remote_checksum_drift_does_not_trigger_a_metadata_reanchor` (`services/codex_desktop/mod.rs:2502`) covers reanchor policy.

There is no `verifying_download` job stage. `ProgressPhase::Verification` is reserved for post-install (`codex-desktop-installer.md:94-96`).

### (b) Remaining checks that still hash, compare identity, or verify install

These are the remaining “校验” surfaces. They do **not** compare to upstream publication fields, but they **do** re-read the whole local installer for SHA-256 (the slow part on large MSIX/DMG).

1. **Streaming local fingerprint at download** — `download.rs:595-687`. SHA-256 is computed over every received chunk and stored on `DownloadedArtifact`.

2. **Full-file revalidate before consume** — `DownloadedArtifact::revalidate` (`download.rs:349-362`) reopens the job file and calls `verify::verify_reader` against the locally stored size/hash.

3. **Prepare / install revalidate** — Windows `package.revalidate_artifact()` immediately before helper pin (`platform/windows/mod.rs:599`); macOS `package.revalidate_artifact()` immediately before `hdiutil` (`platform/macos/dmg.rs:111`). `verify.rs:140-211` is the shared reader: size equality then SHA-256 equality; mismatch → `InstallerErrorCode::ChecksumMismatch` with diagnostic `"artifact checksum did not match expected metadata"` (`verify.rs:309-311`). That “metadata” is the local job fingerprint, not the mirror.

4. **Windows pin + helper handoff (another full-file hash)** — `VerifiedFilePin::open` (`helper.rs:130-133`) and `recheck` (`helper.rs:147-155`) both `verify_reader` the entire file against `package.local_sha256()`.

5. **Windows PackageBridge copy integrity** — `package_bridge.rs:294-335`, `420-422`: copy SHA-256 must equal the locally expected digest; sealed-file rehash. This is same-file mutation detection across the ProgramData bridge, not an upstream checksum.

6. **Disk preflight from size hint** — if `download_size_hint` is present, `ensure_required_disk_space` uses it (`services/codex_desktop/mod.rs:968-980`). Hint is not an equality gate on downloaded bytes.

7. **Post-install existence / operational shape** — after native install the job enters `JobStage::VerifyingInstallation` (`services/codex_desktop/mod.rs:1026-1053`). UI string: `"正在验证安装结果。"` (`CodexDesktopInstallerPanel.tsx:25`) / `"正在确认安装结果"` (`zh.json:3208`). This checks that one installed application exists and has operational identity/platform shape; it does not compare to remote hash.

8. **Windows Stable identity for discovery / launch, not download admit** — constant `WINDOWS_CODEX_STABLE_IDENTITY = "OpenAI.Codex"` (`platform.rs:39`). Used to filter already-installed PackageManager records (`windows/mod.rs:401, 445, 486`). Post-install **new** result uses `installed_application_from_dynamic_record` (`windows/mod.rs:518-543`), which requires nonempty operational identity/publisher/version, not equality with a maintained publisher/hash constant.

9. **macOS Stable bundle ID for discovery** — `MACOS_CODEX_STABLE_IDENTITY = "com.openai.codex"` (`platform.rs:46`). `validate_stable_bundle` (`bundle.rs:489-506`) compares the already-discovered local bundle to that Stable ID. Comment at `bundle.rs:490-492` states publisher / signature / Team ID / architecture / minimum OS are not FyAgent installer admission. Test `stable_discovery_does_not_admit_on_team_architecture_gatekeeper_or_minimum_os` (`bundle.rs:1219`). DMG install still requires exactly one top-level `.app` and local staged/installed identity continuity (`dmg.rs` `verify_installed_replacement`).

10. **Native OS installer trust** — Windows `AddPackageByUriAsync` / PackageManager may still reject signature/trust; mapped to `PackageSignatureInvalid` (`deployment.rs:253-254`, `helper.rs:1073`). Spec: surface native result; do not reinterpret as upstream-field mismatch.

11. **DTO / i18n** — `CHECKSUM_MISMATCH` remains for local same-file mutation (`src/shared/codex-desktop/parsers.ts:81`; `zh.json:3228`: “下载文件在交给安装程序前发生了变化”). No `verifying_download` renderer view.

### Where full-file hashing still runs (slow path)

On a successful Windows one-click install the same local MSIX is hashed:

- once while downloading (`download.rs` hasher);
- again at `revalidate_artifact` before pin (`windows/mod.rs:599`);
- again in `VerifiedFilePin::open` (`helper.rs:133`);
- again in `VerifiedFilePin::recheck` (`helper.rs:155`);
- again when PackageBridge copies/seals (`package_bridge.rs`).

macOS hashes while downloading, then `revalidate_artifact` before mount (`dmg.rs:111`).

These are class (b). Spec currently requires them. They are also the remaining cost/latency that matches the user’s “长时间校验” description, even though they are not upstream checksum maintenance.

### CLI one-click

No Codex CLI installer verification remains because Codex CLI install is not offered. Generic CLI install scripts (e.g. `https://claude.ai/install.sh` in `misc.rs:460`) have no FyAgent checksum/signature admission.

## Related Specs

- `.trellis/spec/backend/codex-desktop-installer.md` — forbids class (a); enumerates class (b) that must remain; wrong vs correct examples at §7.
- `.trellis/spec/backend/codex-provider-configuration.md` — Codex provider TOML; not the desktop installer.
- `.trellis/spec/frontend/v2-agent-models.md` — Codex desktop installer is mounted on the Agent directory, not Models.

## Caveats / Not Found

- No remaining comparison of downloaded bytes to AgentsMirror `sha256`, remote signature, Team ID, or maintained publisher allowlist on the install-admit path.
- Native PackageManager / Gatekeeper / `hdiutil` behavior is not re-proven here; residual risk is already stated in the installer spec §6.
- If item 1 is implemented as “remove all hashing,” that would also remove class (b) local same-file handoff, which the current spec still requires. If item 1 is implemented as “do not maintain upstream checksums,” current code already matches that.
- Codex CLI one-click install does not exist in this tree; only Desktop.