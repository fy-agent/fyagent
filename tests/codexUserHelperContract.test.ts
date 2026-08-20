import fs from "node:fs";
import path from "node:path";
import { parse as parseToml } from "smol-toml";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const HELPER_ROOT = "src-tauri/user-helper";
const PACKAGE_BRIDGE_ROOT =
  "FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}";

function read(relativePath: string): string {
  return fs
    .readFileSync(path.join(ROOT, relativePath), "utf8")
    .replace(/\r\n/gu, "\n");
}

function productionRust(relativePath: string): string {
  return read(relativePath).split("#[cfg(test)]", 1)[0];
}

function section(source: string, start: string, end: string): string {
  const startIndex = source.indexOf(start);
  expect(startIndex, `missing section start: ${start}`).toBeGreaterThanOrEqual(
    0,
  );
  const endIndex = source.indexOf(end, startIndex + start.length);
  expect(endIndex, `missing section end: ${end}`).toBeGreaterThan(startIndex);
  return source.slice(startIndex, endIndex);
}

function expectInOrder(
  source: string,
  markers: readonly string[],
  label: string,
): void {
  let previous = -1;
  for (const marker of markers) {
    const current = source.indexOf(marker, previous + 1);
    expect(current, `${label}: ${marker}`).toBeGreaterThan(previous);
    previous = current;
  }
}

function count(source: string, literal: string): number {
  return source.split(literal).length - 1;
}

const manifest = parseToml(read(`${HELPER_ROOT}/Cargo.toml`)) as Record<
  string,
  any
>;
const cli = productionRust(`${HELPER_ROOT}/src/cli.rs`);
const layout = productionRust(`${HELPER_ROOT}/src/layout.rs`);
const bridgeControl = productionRust(`${HELPER_ROOT}/src/bridge_control.rs`);
const protocol = productionRust(`${HELPER_ROOT}/src/protocol.rs`);
const runtime = productionRust(`${HELPER_ROOT}/src/windows.rs`);
const helperLibrary = productionRust(`${HELPER_ROOT}/src/lib.rs`);
const main = read(`${HELPER_ROOT}/src/main.rs`);
const helperBuild = read(`${HELPER_ROOT}/build.rs`);
const helperManifest = read(
  `${HELPER_ROOT}/windows/fyagent-user-helper.manifest`,
);
const windowsAdapter = read(
  "src-tauri/src/codex_desktop/platform/windows/mod.rs",
);
const windowsDeployment = read(
  "src-tauri/src/codex_desktop/platform/windows/deployment.rs",
);
const packageBridge = productionRust(
  "src-tauri/src/codex_desktop/platform/windows/package_bridge.rs",
);
const downloadedArtifact = read("src-tauri/src/codex_desktop/download.rs");
const platformCore = read("src-tauri/src/codex_desktop/platform.rs");
const staging = read("src-tauri/src/codex_desktop/temp.rs");
const parentHelper = productionRust(
  "src-tauri/src/codex_desktop/platform/windows/helper.rs",
);
const processLaunch = read("src-tauri/src/platform/process_launch.rs");
const explorerLaunch = read(
  "src-tauri/src/platform/windows/interactive_user.rs",
);
const prepareHelper = read("scripts/prepare-windows-user-helper.mjs");
const windowsTauriConfig = JSON.parse(
  read("src-tauri/tauri.windows.conf.json"),
) as Record<string, any>;

describe("Codex current-user helper static contract", () => {
  it("keeps the independent helper crate minimal and removes the HTTP source", () => {
    expect(manifest.features?.default).toEqual([]);
    expect(new Set(manifest.features?.["helper-runtime"])).toEqual(
      new Set(["dep:windows", "dep:windows-future"]),
    );

    const binaries = manifest.bin as Array<Record<string, unknown>>;
    expect(binaries).toHaveLength(1);
    expect(binaries[0]).toMatchObject({
      name: "fyagent-user-helper",
      path: "src/main.rs",
      "required-features": ["helper-runtime"],
    });

    const windowsDependencies =
      manifest.target?.['cfg(target_os = "windows")']?.dependencies;
    expect(Object.keys(windowsDependencies).sort()).toEqual([
      "windows",
      "windows-future",
    ]);
    for (const dependency of Object.values(windowsDependencies) as Array<
      Record<string, unknown>
    >) {
      expect(dependency.optional).toBe(true);
    }

    expect(JSON.stringify(manifest)).not.toMatch(
      /tauri|\burl\b|Win32_Networking_WinSock/iu,
    );
    expect(
      fs.existsSync(path.join(ROOT, HELPER_ROOT, "src/loopback_http.rs")),
    ).toBe(false);
    expect(
      fs.existsSync(path.join(ROOT, HELPER_ROOT, "src/source_control.rs")),
    ).toBe(false);
    expect(helperLibrary).not.toMatch(/loopback_http|source_control/iu);
    expect(main).toMatch(/#\[cfg\(target_os = "windows"\)\]\s+mod windows;/u);
    expect(main).toContain(
      '#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]',
    );
  });

  it("accepts only the fixed action, canonical job ID, and pipe nonce", () => {
    expect(cli).toContain('INSTALL_ACTION: &str = "codex-msix-install"');
    expect(cli).toContain('JOB_ID_FLAG: &str = "--job-id"');
    expect(cli).toContain('PIPE_FLAG: &str = "--pipe"');
    expect(cli).toMatch(/PIPE_NONCE_BYTES:\s*usize\s*=\s*64\s*;/u);
    expect(cli).toMatch(/JOB_ID_BYTES:\s*usize\s*=\s*36\s*;/u);
    expect(cli).toContain("Vec::with_capacity(5)");
    expect(cli).toMatch(/if raw\[0\] != INSTALL_ACTION/u);
    expect(cli).toMatch(/if raw\[1\] != JOB_ID_FLAG/u);
    expect(cli).toMatch(/if raw\[3\] != PIPE_FLAG/u);

    expect(cli.match(/"--[a-z-]+"/gu)).toEqual(['"--job-id"', '"--pipe"']);
    expect(cli).not.toMatch(
      /--(?:path|uri|root|operation|mode|port|scope)|current_dir|temp_dir|std::env::var/u,
    );
  });

  it("owns every fixed bridge component once and versions the local controls", () => {
    for (const contract of [
      'INSTALLER_FILE_NAME: &str = "installer.msix"',
      'PACKAGE_BRIDGE_PART_FILE_NAME: &str = "installer.msix.part"',
      `PACKAGE_BRIDGE_ROOT_DIRECTORY: &str =\n    "${PACKAGE_BRIDGE_ROOT}"`,
      'PACKAGE_BRIDGE_VERSION_DIRECTORY: &str = "v1"',
      String.raw`USER_HELPER_PIPE_PREFIX: &str = r"\\.\pipe\LOCAL\FyAgent.UserHelper.v2."`,
      String.raw`USER_HELPER_ADMISSION_EVENT_PREFIX: &str = r"Local\FyAgent.UserHelper.Admit.v2."`,
      String.raw`USER_HELPER_CANCEL_EVENT_PREFIX: &str = r"Local\FyAgent.UserHelper.Cancel.v2."`,
    ]) {
      expect(layout).toContain(contract);
    }

    const fixedSources = `${layout}\n${packageBridge}\n${runtime}`;
    for (const literal of [
      PACKAGE_BRIDGE_ROOT,
      '"installer.msix.part"',
      '"installer.msix"',
    ]) {
      expect(count(fixedSources, literal), literal).toBe(1);
    }
    expect(packageBridge).toContain("PACKAGE_BRIDGE_ROOT_DIRECTORY");
    expect(packageBridge).toContain("PACKAGE_BRIDGE_VERSION_DIRECTORY");
    expect(packageBridge).toContain("PACKAGE_BRIDGE_PART_FILE_NAME");
    expect(packageBridge).toContain("INSTALLER_FILE_NAME");
    expect(runtime).toContain("PACKAGE_BRIDGE_ROOT_DIRECTORY");
    expect(runtime).toContain("PACKAGE_BRIDGE_VERSION_DIRECTORY");
    expect(runtime).toContain("INSTALLER_FILE_NAME");
    expect(runtime).not.toContain("derive_install_layout(");
  });

  it("pins protocol v2 and the explicit Hello-control-Started-admission state machine", () => {
    expect(protocol).toMatch(/PROTOCOL_VERSION:\s*u8\s*=\s*2\s*;/u);
    expect(protocol).toMatch(/FRAME_LENGTH_BYTES:\s*usize\s*=\s*4\s*;/u);
    expect(protocol).toMatch(/MAX_PROTOCOL_MESSAGES:\s*usize\s*=\s*104\s*;/u);
    expect(protocol).toMatch(/MAX_ERROR_MESSAGE_BYTES:\s*usize\s*=\s*256\s*;/u);

    const messageEnum = protocol.match(
      /pub enum HelperMessage\s*\{([\s\S]*?)\n\}/u,
    )?.[1];
    expect(messageEnum).toBeDefined();
    expect(messageEnum).toMatch(/\bHello\b/u);
    expect(messageEnum).toMatch(
      /\bStarted\b[\s\S]*package:\s*PinnedPackageIdentity/u,
    );
    expect(messageEnum).toMatch(/\bProgress\b[\s\S]*completed:\s*u8/u);
    expect(messageEnum).toMatch(/\bSuccess\b/u);
    expect(messageEnum).toMatch(
      /\bError\b[\s\S]*code:\s*HelperErrorCode[\s\S]*message:\s*String/u,
    );
    expect(messageEnum).not.toMatch(/Path|Command|Uri|Scope|OperationId/iu);

    const phases = protocol.match(
      /enum ProtocolPhase\s*\{([\s\S]*?)\n\}/u,
    )?.[1];
    expect(phases).toBeDefined();
    for (const phase of [
      "AwaitingHello",
      "AwaitingControl",
      "AwaitingStarted",
      "AwaitingAdmission",
      "Running",
      "Terminal",
    ]) {
      expect(phases).toMatch(new RegExp(`\\b${phase}\\b`, "u"));
    }

    const sequence = protocol.slice(
      protocol.indexOf("impl HelperProtocolSequence"),
    );
    expect(sequence).toMatch(
      /AwaitingHello, HelperMessage::Hello[\s\S]+?AwaitingControl/u,
    );
    expect(sequence).toMatch(
      /fn mark_control_sent[\s\S]+?AwaitingControl[\s\S]+?AwaitingStarted/u,
    );
    expect(sequence).toMatch(
      /AwaitingStarted, HelperMessage::Started[\s\S]+?AwaitingAdmission/u,
    );
    expect(sequence).toMatch(
      /fn mark_admitted[\s\S]+?AwaitingAdmission[\s\S]+?Running/u,
    );
    expect(sequence).toMatch(
      /ProtocolPhase::Running, HelperMessage::Progress/u,
    );
    expect(sequence).toMatch(
      /ProtocolPhase::Running, HelperMessage::Success[\s\S]+?ProtocolPhase::Terminal/u,
    );

    const mutations = [
      protocol.replace("PROTOCOL_VERSION: u8 = 2", "PROTOCOL_VERSION: u8 = 1"),
      protocol.replace(
        "self.phase = ProtocolPhase::AwaitingControl;",
        "self.phase = ProtocolPhase::AwaitingStarted;",
      ),
      protocol.replace(
        "self.phase = ProtocolPhase::AwaitingAdmission;",
        "self.phase = ProtocolPhase::Running;",
      ),
    ];
    const hasV2Ordering = (source: string) =>
      source.includes("PROTOCOL_VERSION: u8 = 2") &&
      source.includes("fn mark_control_sent") &&
      source.includes("self.phase = ProtocolPhase::AwaitingControl;") &&
      source.includes("self.phase = ProtocolPhase::AwaitingStarted;") &&
      source.includes("fn mark_admitted") &&
      source.includes("self.phase = ProtocolPhase::AwaitingAdmission;") &&
      source.includes("self.phase = ProtocolPhase::Running;");
    expect(hasV2Ordering(protocol)).toBe(true);
    for (const mutation of mutations)
      expect(hasV2Ordering(mutation)).toBe(false);
  });

  it("does not misclassify a package downgrade as a package-in-use failure", () => {
    const helperMapping = section(
      protocol,
      "pub fn helper_error_code_for_deployment_hresult",
      "\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
    );
    expect(helperMapping).toMatch(
      /0x8007_3D02\s*=>\s*HelperErrorCode::PackageInUse/u,
    );
    expect(helperMapping).toMatch(
      /0x8007_3D06\s*=>\s*HelperErrorCode::PackageDowngrade/u,
    );

    const parentMapping = section(
      windowsDeployment,
      "fn map_deployment_hresult",
      '\n}\n\n#[cfg(target_os = "windows")]',
    );
    expect(parentMapping).toMatch(
      /0x8007_3D02\s*=>\s*InstallerErrorCode::WindowsPackageInUse/u,
    );
    expect(parentMapping).toMatch(
      /0x8007_3D06\s*=>\s*InstallerErrorCode::MetadataChanged/u,
    );

    const parentHelperMapping = section(
      parentHelper,
      "fn map_helper_error",
      "\n}\n\nfn wait_for_overlapped",
    );
    expect(parentHelperMapping).toMatch(
      /HelperErrorCode::PackageDowngrade\s*=>\s*InstallerErrorCode::MetadataChanged/u,
    );
  });

  it("locks the exact 80-byte pathless FYABRIDG control and rejects mutations", () => {
    expect(bridgeControl).toMatch(/BRIDGE_CONTROL_VERSION:\s*u8\s*=\s*2/u);
    expect(bridgeControl).toMatch(/BRIDGE_CONTROL_BYTES:\s*usize\s*=\s*80/u);
    expect(bridgeControl).toMatch(
      /BRIDGE_OPERATION_ID_BYTES:\s*usize\s*=\s*32/u,
    );
    expect(bridgeControl).toContain('const MAGIC: [u8; 8] = *b"FYABRIDG"');
    for (const offset of [
      "VERSION_OFFSET: usize = 8",
      "RESERVED_START: usize = 9",
      "VOLUME_OFFSET: usize = 24",
      "FILE_INDEX_OFFSET: usize = 32",
      "SIZE_OFFSET: usize = 40",
      "OPERATION_ID_OFFSET: usize = 48",
    ]) {
      expect(bridgeControl).toContain(offset);
    }

    const record = section(
      bridgeControl,
      "pub struct PackageBridgeControl",
      "fn append_lower_hex",
    );
    expect(record).toMatch(/operation_id:\s*BridgeOperationId/u);
    expect(record).toMatch(/package:\s*PinnedPackageIdentity/u);
    expect(record).toMatch(/bytes\[RESERVED_START\.\.VOLUME_OFFSET\]/u);
    expect(record).toMatch(/\.any\(\|byte\| \*byte != 0\)/u);
    expect(record).not.toMatch(
      /PathBuf|std::path|\bUri\b|\bhost\b|\bport\b|\bmode\b|\bhash\b|String/u,
    );
    expect(bridgeControl).toContain("BridgeOperationId([redacted])");
    expect(bridgeControl).toContain("directory_name(self) -> String");
    expect(bridgeControl).toContain("append_lower_hex");
    expect(bridgeControl).not.toContain("FYAHHTTP");

    const acceptsExactControl = (source: string) =>
      source.includes("BRIDGE_CONTROL_BYTES: usize = 80") &&
      source.includes("BRIDGE_CONTROL_VERSION: u8 = 2") &&
      source.includes('*b"FYABRIDG"') &&
      source.includes("RESERVED_START: usize = 9") &&
      source.includes("VOLUME_OFFSET: usize = 24") &&
      source.includes("OPERATION_ID_OFFSET: usize = 48");
    expect(acceptsExactControl(bridgeControl)).toBe(true);
    for (const mutation of [
      bridgeControl.replace("FYABRIDG", "FYAHHTTP"),
      bridgeControl.replace(
        "BRIDGE_CONTROL_BYTES: usize = 80",
        "BRIDGE_CONTROL_BYTES: usize = 79",
      ),
      bridgeControl.replace(
        "BRIDGE_CONTROL_VERSION: u8 = 2",
        "BRIDGE_CONTROL_VERSION: u8 = 1",
      ),
      bridgeControl.replace(
        "VOLUME_OFFSET: usize = 24",
        "VOLUME_OFFSET: usize = 16",
      ),
      bridgeControl.replace(
        "OPERATION_ID_OFFSET: usize = 48",
        "OPERATION_ID_OFFSET: usize = 40",
      ),
    ]) {
      expect(acceptsExactControl(mutation)).toBe(false);
    }
  });

  it("makes the helper prove the controlled bridge before admission and progress", () => {
    const runInstall = section(
      runtime,
      "pub(crate) fn run_install",
      "\nfn deploy_fixed_package",
    );
    expectInOrder(
      runInstall,
      [
        "ParentControls::open",
        "PipeChannel::connect",
        "channel.send_hello()",
        "channel.read_bridge_control",
        "PinnedPackageFile::open(bridge_control)",
        "package_pin.recheck_for_helper()",
        "channel.send_started",
        "controls.wait_for_admission",
        "channel.mark_admitted()",
        "channel.send_progress(0)",
        "deploy_fixed_package",
      ],
      "helper admission ordering",
    );
    expect(
      runInstall.slice(0, runInstall.indexOf("controls.wait_for_admission")),
    ).not.toContain("send_progress(");

    const channel = section(runtime, "impl PipeChannel", "\nfn write_message");
    for (const state of [
      "Initial",
      "HelloSent",
      "ControlReceived",
      "Started",
      "Terminal",
    ]) {
      expect(runtime).toMatch(new RegExp(`\\b${state}\\b`, "u"));
    }
    expect(channel).toMatch(
      /fn send_hello[\s\S]+?ChannelState::Initial[\s\S]+?HelperMessage::Hello[\s\S]+?ChannelState::HelloSent/u,
    );
    expect(channel).toMatch(
      /fn read_bridge_control[\s\S]+?ChannelState::HelloSent[\s\S]+?PackageBridgeControl::decode[\s\S]+?ChannelState::ControlReceived/u,
    );
    expect(channel).toMatch(
      /fn send_started[\s\S]+?ChannelState::ControlReceived[\s\S]+?HelperMessage::Started/u,
    );
    expect(channel).toMatch(/fn send_progress[\s\S]+?admitted: true/u);
    expect(channel).toMatch(
      /fn send_prestart_error[\s\S]+?HelloSent \| ChannelState::ControlReceived/u,
    );
  });

  it("resolves and pins only the fixed protected ProgramData file", () => {
    for (const boundary of [
      "SHGetKnownFolderPath",
      "FOLDERID_ProgramData",
      "CoTaskMemFree",
      "GetVolumePathNameW",
      "GetDriveTypeW",
      "DRIVE_FIXED",
      "GetVolumeInformationW",
      "FILE_PERSISTENT_ACLS",
      "NtCreateFile",
      "OBJ_DONT_REPARSE",
      "FILE_OPEN_REPARSE_POINT",
      "GetFileInformationByHandleEx",
      "FileStandardInfo",
      "FileAttributeTagInfo",
      "FILE_ATTRIBUTE_OFFLINE",
      "FILE_ATTRIBUTE_RECALL_ON_OPEN",
      "FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS",
      "GetSecurityInfo",
      "SE_FILE_OBJECT",
      "GetSecurityDescriptorControl",
      "SE_DACL_PROTECTED",
      "GetAclInformation",
      "GetAce",
      "AccessCheck",
      "MAXIMUM_ALLOWED",
    ]) {
      expect(runtime, boundary).toContain(boundary);
    }

    const packagePin = section(
      runtime,
      "struct PinnedPackageFile",
      "\nfn native_identity",
    );
    for (const fixedComponent of [
      "PACKAGE_BRIDGE_ROOT_DIRECTORY",
      "PACKAGE_BRIDGE_VERSION_DIRECTORY",
      "control.operation_id().directory_name()",
      "INSTALLER_FILE_NAME",
    ]) {
      expect(packagePin).toContain(fixedComponent);
    }
    expect(packagePin).toContain("verify_exact_bridge_acl");
    expect(packagePin).toContain("verify_effective_access");
    expect(packagePin).toContain("BRIDGE_DIRECTORY_DANGEROUS_ACCESS");
    expect(packagePin).toContain("BRIDGE_FILE_DANGEROUS_ACCESS");
    expect(packagePin).toContain("native_identity(control.package())");
    expect(packagePin).toContain("uri_reopen");
    expect(packagePin).not.toMatch(
      /CACHE_DIRECTORY|CODEX_INSTALLER_DIRECTORY|job_id|sha256|verify_reader|expected_sha/iu,
    );

    expect(runtime).toMatch(/NumberOfLinks\s*!=\s*1/u);
    expect(runtime).toMatch(/DeletePending/u);
    const helperAcl = section(
      runtime,
      "fn verify_exact_bridge_acl",
      "\nfn trustee_matches",
    );
    for (const exactBoundary of [
      "SE_OWNER_DEFAULTED",
      "SE_GROUP_DEFAULTED",
      "SE_DACL_DEFAULTED",
      "SE_DACL_AUTO_INHERIT_REQ",
      "SE_DACL_AUTO_INHERITED",
      "ACL_REVISION",
      "acl.Sbz1 != 0",
      "acl.Sbz2 != 0",
      "information.AceCount != expected.len()",
      "header.AceFlags != 0",
      "sid_length != encoded_sid_length",
      "length != ace_size",
    ]) {
      expect(helperAcl, exactBoundary).toContain(exactBoundary);
    }
    expect(helperAcl).not.toMatch(/AclBytesFree|AclBytesInUse/u);
    expect(runtime).toContain("CheckTokenMembership");
    expect(runtime).toContain("is_local_administrator");
    expect(runtime).toContain("forbidden_access_rejected");
    expect(runtime).not.toMatch(
      /std::env::(?:var|var_os|current_dir|temp_dir)|C:\\+ProgramData/iu,
    );
    expect(runtime).not.toMatch(/SetSecurityInfo|SetKernelObjectSecurity/u);
  });

  it("round-trips one local DOS file URI and exposes only AddPackage", () => {
    const uriRoundtrip = section(
      runtime,
      "fn local_file_uri_roundtrip",
      "\nfn has_local_file_uri_shape",
    );
    expectInOrder(
      uriRoundtrip,
      ["UrlCreateFromPathW", "PathCreateFromUrlW", "roundtrip_length"],
      "DOS file URI round trip",
    );
    expect(uriRoundtrip).toContain(
      "roundtrip[..roundtrip_length] != *original",
    );
    expect(runtime).toContain("validate_ordinary_dos_path");
    expect(runtime).toContain('scheme != "file"');
    expect(runtime).toMatch(
      /!host\.is_empty\(\)[\s\S]+?!query\.is_empty\(\)[\s\S]+?!fragment\.is_empty\(\)/u,
    );
    expect(runtime.match(/\.AddPackageByUriAsync\s*\(/gu)).toHaveLength(1);
    expect(runtime).not.toMatch(
      /StagePackage|RegisterPackage|ProvisionPackage|RequestAddPackage|PackageVolume/iu,
    );
    expect(runtime).not.toMatch(
      /std::process::Command|Command::new|CreateProcess|ShellExecute|cmd\.exe|powershell|tauri/iu,
    );

    const helperBridgeSource = `${runtime}\n${helperLibrary}`;
    for (const retiredSource of [
      "FYAHHTTP",
      "127.0.0.1",
      "PackageSourceControl",
      "SOURCE_CONTROL_BYTES",
      "source_control",
      "loopback_http",
      "TcpListener",
      "WinSock",
      "Accept-Ranges",
      "Content-Range",
      '"GET"',
      '"HEAD"',
    ]) {
      expect(helperBridgeSource, retiredSource).not.toContain(retiredSource);
    }
  });

  it("creates the bridge atomically with exact stable and Alice-specific ACLs", () => {
    for (const boundary of [
      "SHGetKnownFolderPath",
      "FOLDERID_ProgramData",
      "GetDriveTypeW",
      "DRIVE_FIXED",
      "GetVolumeInformationW",
      "FILE_PERSISTENT_ACLS",
      "GetDiskFreeSpaceExW",
      "NtCreateFile",
      "OBJ_DONT_REPARSE",
      "SecurityDescriptor:",
      "FILE_CREATE",
      "FILE_OPEN_IF",
      "GetSecurityInfo",
      "SE_FILE_OBJECT",
      "GetSecurityDescriptorControl",
      "SE_DACL_PROTECTED",
      "GetAclInformation",
      "GetAce",
      "AccessCheck",
      "MAXIMUM_ALLOWED",
      "FILE_DELETE_CHILD",
    ]) {
      expect(packageBridge, boundary).toContain(boundary);
    }
    expect(packageBridge).not.toMatch(/SetSecurityInfo|SetNamedSecurityInfo/u);
    expect(packageBridge).toMatch(
      /STABLE_DIRECTORY_ACES[\s\S]+?AuthenticatedUsers/u,
    );
    expect(packageBridge).toMatch(/OPERATION_DIRECTORY_ACES[\s\S]+?ShellUser/u);
    expect(packageBridge).toMatch(/PACKAGE_LEAF_ACES[\s\S]+?ShellUser/u);
    expect(packageBridge).toMatch(/AceFlags\s*!=\s*0/u);
    expect(packageBridge).toMatch(/AceCount\s*!=\s*expected\.len\(\)/u);
    expect(packageBridge).toMatch(/WinBuiltinAdministratorsSid/u);
    const parentAcl = section(
      packageBridge,
      "fn verify_exact_descriptor",
      "\nstruct OwnedHandle",
    );
    for (const exactBoundary of [
      "SE_OWNER_DEFAULTED",
      "SE_GROUP_DEFAULTED",
      "SE_DACL_DEFAULTED",
      "SE_DACL_AUTO_INHERIT_REQ",
      "SE_DACL_AUTO_INHERITED",
      "ACL_REVISION",
      "acl.Sbz1 != 0",
      "acl.Sbz2 != 0",
      "information.AceCount != expected.len()",
      "ace.Header.AceFlags != 0",
      "sid_length != encoded_sid_length",
      "length != ace_size",
    ]) {
      expect(parentAcl, exactBoundary).toContain(exactBoundary);
    }
    expect(parentAcl).not.toMatch(/AclBytesFree|AclBytesInUse/u);

    const stableDescriptor = section(
      packageBridge,
      "DescriptorKind::StableDirectory => format!",
      "DescriptorKind::OperationDirectory => format!",
    );
    expect(stableDescriptor).toContain(";;;BA");
    expect(stableDescriptor).toContain(";;;SY");
    expect(stableDescriptor).toContain(";;;AU");
    expect(stableDescriptor).not.toContain("{shell_sid}");

    const shellToken = section(
      packageBridge,
      "fn shell_access_check_token",
      "\nfn effective_file_access",
    );
    expectInOrder(
      shellToken,
      [
        "GetShellWindow",
        "GetWindowThreadProcessId",
        "OpenProcessToken",
        "GetTokenInformation",
        "EqualSid",
        "DuplicateToken",
      ],
      "real Alice access-check token",
    );
    const dangerousRights = section(
      packageBridge,
      "let dangerous = FILE_DELETE_CHILD.0",
      "if ancestor_mutation_rejected",
    );
    for (const right of [
      "FILE_DELETE_CHILD",
      "DELETE",
      "WRITE_DAC",
      "WRITE_OWNER",
      "FILE_WRITE_EA",
      "FILE_WRITE_ATTRIBUTES",
    ]) {
      expect(dangerousRights).toContain(right);
    }
    expect(packageBridge).toContain("CheckTokenMembership");
    expect(packageBridge).toContain("token_is_local_administrator");
    expect(packageBridge).toContain("ancestor_mutation_rejected");
  });

  it("copies, hashes, flushes, renames without replacement, and reopens the final leaf", () => {
    const createBridge = section(
      packageBridge,
      "pub(super) fn create",
      "pub(super) const fn control",
    );
    expectInOrder(
      createBridge,
      [
        "native_file_identity(source_file",
        "create_package_leaf",
        "copy_exact_from_source",
        "FlushFileBuffers",
        "rename_leaf_without_replacement",
        "open_final_package_leaf",
        "hash_exact_file",
        "native_file_identity(source_file",
        "PackageBridgeControl::new",
        "bridge.recheck()",
      ],
      "parent bridge sealing",
    );
    expect(packageBridge).toContain("Sha256::new()");
    expect(packageBridge).toContain("number_of_links != 1");
    expect(packageBridge).toContain("FileStandardInfo");
    expect(packageBridge).toContain("DeletePending");
    expect(packageBridge).toContain("FILE_ATTRIBUTE_OFFLINE");
    expect(packageBridge).toContain("FILE_ATTRIBUTE_RECALL_ON_OPEN");
    expect(packageBridge).toContain("FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS");
    expect(packageBridge).toMatch(/ReplaceIfExists\s*=\s*false/u);
    expect(packageBridge).not.toMatch(
      /std::fs::copy|fs::copy|CopyFileW|replace_existing\s*:\s*true/u,
    );

    const exactCopy = section(
      packageBridge,
      "fn copy_exact_from_source",
      "\nfn write_all_at",
    );
    expect(exactCopy).toContain("while offset < expected_size");
    expect(exactCopy).toContain("seek_read");
    expect(exactCopy).toContain("write_all_at");
    expect(exactCopy).toContain("hasher.update");
    expect(exactCopy).toMatch(/seek_read\(&mut trailing, expected_size\)/u);
  });

  it("keeps bridge cleanup handle-relative, known-only, and outside NSIS", () => {
    const cleanup = section(
      packageBridge,
      "pub(super) fn cleanup",
      "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\nenum NativeObjectKind",
    );
    expect(cleanup).toContain("INSTALLER_FILE_NAME");
    expect(cleanup).toContain("open_relative(");
    expect(cleanup).toContain("mark_handle_for_deletion");
    expect(cleanup).toContain("self.operation.file");
    expect(cleanup).not.toMatch(
      /remove_dir_all|read_dir|walkdir|glob|wildcard/iu,
    );
    expect(packageBridge).not.toMatch(
      /remove_dir_all|WalkDir|std::fs::remove_/u,
    );
    expect(packageBridge).toMatch(
      /fn (?:cleanup_bounded_orphans|cleanup_known_orphans|cleanup_stale_operations|cleanup_orphans)/u,
    );
    expect(packageBridge).toMatch(
      /64[\s\S]+?lowercase|canonical[\s\S]+?operation/u,
    );
  });

  it("authenticates Hello before sending control and admits only the same bridge identity", () => {
    const runner = section(
      parentHelper,
      "fn run_pinned_user_helper",
      "\nfn generate_nonce",
    );
    expectInOrder(
      runner,
      [
        "pin.recheck()",
        "pin.duplicate_source_file()",
        "ProtectedPackageBridge::create",
        "ParentControlEvents::create",
        "OneShotPipeServer::create",
        "PinnedHelperImage::open",
        "launch_fyagent_user_helper_as_user",
        "read_frame(first_frame_timeout)",
        "validate_client",
        "decode_protocol_frame",
        "HelperProtocolAction::Hello",
        "bridge().recheck()",
        ".send_bridge_control(",
        "sequence.mark_control_sent()",
        "started_message",
        "HelperProtocolAction::Started",
        "bridge_identity_matches",
        "revalidate_interactive_user_context(context)",
        "bridge().recheck()",
        "controls().admit()",
        "lifetime.mark_admitted()",
        "sequence.mark_admitted()",
        "consume_protocol",
      ],
      "parent helper admission ordering",
    );
    expect(runner).not.toMatch(/ParentPackageSource|source_control|WinSock/iu);

    for (const boundary of [
      "FILE_FLAG_FIRST_PIPE_INSTANCE",
      "PIPE_TYPE_MESSAGE",
      "PIPE_READMODE_MESSAGE",
      "PIPE_REJECT_REMOTE_CLIENTS",
      "GetNamedPipeClientProcessId",
      "GetNamedPipeClientSessionId",
      "ImpersonateNamedPipeClient",
      "OpenThreadToken",
      "RevertToSelf",
    ]) {
      expect(parentHelper).toContain(boundary);
    }
    expect(parentHelper).toContain("BCryptGenRandom");
    expect(parentHelper).toMatch(/let mut random = \[0_u8; 32\]/u);
    expect(parentHelper).toContain(
      "O:BAG:BAD:P(A;;0x0012008b;;;{shell_sid})(A;;RC;;;SY)(A;;RC;;;BA)",
    );
  });

  it("retains source and bridge after unknown admission and cleans only a settled close", () => {
    const admittedProcess = section(
      parentHelper,
      "struct AdmittedHelperProcess",
      "\nstruct HelperLifetime",
    );
    expect(admittedProcess).toContain("_process_handle: OwnedWin32Handle");

    const lifetime = section(
      parentHelper,
      "struct HelperLifetime",
      "\nstatic HELPER_GATE",
    );
    expect(lifetime).toContain("process: Option<AdmittedHelperProcess>");
    expect(lifetime).toMatch(
      /pin:\s*Option<Box<dyn WindowsVerifiedFilePin>>[\s\S]+?bridge:\s*Option<ProtectedPackageBridge>/u,
    );
    expect(lifetime).toMatch(/admitted:\s*bool[\s\S]+?settled:\s*bool/u);
    expect(lifetime).toMatch(
      /if self\.admitted && !self\.settled[\s\S]+?retain_quarantined_lifetime\(Self \{[\s\S]+?pin: self\.pin\.take\(\)[\s\S]+?bridge: self\.bridge\.take\(\)/u,
    );

    const consume = section(
      parentHelper,
      "fn consume_protocol",
      "\nfn wait_for_clean_terminal_close",
    );
    expect(consume).toContain("wait_for_clean_terminal_close");
    expect(consume).toMatch(
      /HelperProtocolAction::Success[\s\S]+?HelperProtocolAction::Failure/u,
    );
    const close = section(
      parentHelper,
      "fn wait_for_clean_terminal_close",
      "\nfn accept_protocol_message",
    );
    expect(close).toMatch(/PipeMessageRead::Closed => Ok\(\(\)\)/u);
    expect(close).toContain("sent data after its terminal message");

    const finish = section(
      parentHelper,
      "fn finish_settled",
      "\nfn fail_before_admission",
    );
    expectInOrder(
      finish,
      [
        "lifetime.mark_settled()",
        "protocol_terminal_result",
        "lifetime.cleanup_bridge()",
        "gate.finish()",
      ],
      "settled cleanup",
    );
    const cancelAndQuarantine = section(
      parentHelper,
      "fn cancel_and_quarantine",
      "\nfn remaining_until",
    );
    expect(cancelAndQuarantine).toContain("debug_assert!(lifetime.admitted)");
    expectInOrder(
      cancelAndQuarantine,
      ["lifetime.controls().cancel()", "gate.quarantine(lifetime"],
      "post-admission failure quarantine",
    );
    expect(cancelAndQuarantine).not.toMatch(
      /finish_settled|cleanup_bridge|mark_settled/u,
    );
    const admittedOutcome = section(
      parentHelper,
      "match consume_protocol(",
      "\n}\n\nfn generate_nonce",
    );
    expect(admittedOutcome).toMatch(
      /Ok\(terminal\) => finish_settled\(gate, lifetime, terminal\)[\s\S]+?Err\(error\) => cancel_and_quarantine\(gate, lifetime, error\)/u,
    );
    expect(parentHelper).not.toMatch(
      /drain_after_cancel|DrainSettlement|cancel_drain/u,
    );
    expect(parentHelper).not.toMatch(/GetExitCodeProcess|TerminateProcess/u);
  });

  it("probes the actual ProgramData volume during Windows preflight", () => {
    const preflight = section(
      windowsAdapter,
      "fn preflight(\n",
      "\nfn install_current_user",
    );
    expect(preflight).toContain("validate_release_for_host");
    expect(preflight).toContain("temp_root.is_dir()");
    expect(preflight).toContain("PlatformInstallPlan::new(vec![");
    expect(preflight).toContain(
      "package_bridge::program_data_bridge_probe_path()?",
    );
    expect(preflight).not.toMatch(/PathBuf::from\("C:|deployment_volume/u);
  });

  it("pins the install-root source handle before creating the protected copy", () => {
    const platformPin = section(
      platformCore,
      "pub(crate) fn open_artifact_for_pinning",
      "\n    pub(crate) fn locked_release",
    );
    expect(platformPin).toMatch(/-> Result<std::fs::File, InstallerError>/u);
    expect(platformPin).toContain(".open_for_read()");
    expect(platformPin).not.toMatch(/artifact_path|File::open|CreateFileW/u);

    const artifactOpen = section(
      downloadedArtifact,
      "pub(crate) fn open_for_read",
      "\n    fn from_completed_download",
    );
    expect(artifactOpen).toContain(
      ".open_final_artifact_for_read(self.artifact_kind)",
    );
    expect(artifactOpen).not.toMatch(/File::open|CreateFileW/u);

    const capabilityOpen = section(
      staging,
      "pub(crate) fn open_final_artifact_for_read",
      "\n    /// Re-proves the frozen/direct-child",
    );
    expect(capabilityOpen).toContain("kind.fixed_local_file_name()");
    expect(capabilityOpen).toContain("WindowsRelativeFileAccess::OpenForRead");

    const pinOpen = section(
      parentHelper,
      "impl VerifiedFilePin",
      "\nimpl WindowsVerifiedFilePin for VerifiedFilePin",
    );
    expect(pinOpen).toContain("package.open_artifact_for_pinning()?");
    expect(pinOpen).toContain("verify_reader(");
    expect(pinOpen).toContain("expected_size");
    expect(pinOpen).toContain("expected_sha256");
    expect(pinOpen).not.toMatch(/artifact_path|File::open|CreateFileW/u);
  });

  it("embeds, builds, and packages one fixed ordinary-user helper", () => {
    expect(helperManifest).toMatch(
      /<requestedExecutionLevel\s+level="asInvoker"\s+uiAccess="false"\s*\/>/u,
    );
    expect(helperManifest).not.toMatch(
      /requireAdministrator|highestAvailable/u,
    );
    expect(read(`${HELPER_ROOT}/windows/fyagent-user-helper.rc`).trim()).toBe(
      '1 24 "fyagent-user-helper.manifest"',
    );
    expect(helperBuild).toContain(
      'embed_resource::ParamsIncludeDirs(&["windows"])',
    );
    expect(helperBuild).toContain(".manifest_required()");

    const supportedTargetBlock = prepareHelper.match(
      /const SUPPORTED_TARGETS = new Set\(\[([\s\S]*?)\]\);/u,
    )?.[1];
    expect(supportedTargetBlock?.match(/[a-z0-9_]+-pc-windows-msvc/gu)).toEqual(
      ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"],
    );
    expect(prepareHelper).toMatch(/SUPPORTED_TARGETS\.has\(target\)/u);
    const prepareImports = [
      ...prepareHelper.matchAll(/(?:\bfrom\s+|^import\s+)["']([^"']+)["']/gmu),
    ].map((match) => match[1]);
    expect(prepareImports.length).toBeGreaterThan(0);
    expect(
      prepareImports.every((specifier) => specifier.startsWith("node:")),
    ).toBe(true);
    expect(prepareHelper).not.toMatch(/\b(?:import|require)\s*\(/u);
    expect(prepareHelper).not.toContain("smol-toml");
    expect(prepareHelper).toContain(
      "must be supplied by trusted Windows build orchestration",
    );
    expect(prepareHelper).toContain("process.execPath");
    expect(prepareHelper).toMatch(
      /\[path\.join\(ROOT, "scripts", "version\.mjs"\), "check"\]/u,
    );
    expect(prepareHelper).toContain("^FyAgent version contract OK:");
    expect(prepareHelper).toContain("(?:0|[1-9]\\d*)");
    expect(prepareHelper).toContain(
      "version contract check emitted unexpected stderr",
    );
    expect(prepareHelper).toContain(
      "version contract check emitted unexpected stdout",
    );
    expect(prepareHelper).toContain("process.stdout.write(result.stdout)");
    expect(prepareHelper).toContain("process.stderr.write(result.stderr)");
    expect(prepareHelper).toMatch(/"--locked"/u);
    expect(prepareHelper).toMatch(/"--features"\s*,\s*"helper-runtime"/u);
    expect(prepareHelper).toMatch(/"--bin"\s*,\s*"fyagent-user-helper"/u);
    expect(count(prepareHelper, "spawnSync(")).toBe(2);
    expect(count(prepareHelper, "shell: false")).toBe(2);
    expectInOrder(
      prepareHelper,
      [
        "fs.copyFileSync(source, temporary, fs.constants.COPYFILE_EXCL)",
        "fs.renameSync(temporary, destination)",
      ],
      "atomic helper sidecar staging",
    );

    expect(windowsTauriConfig.bundle?.externalBin).toEqual([
      "binaries/fyagent-user-helper",
    ]);
    expect(windowsTauriConfig.build?.beforeDevCommand).toContain(
      "node scripts/prepare-windows-user-helper.mjs",
    );
    expect(windowsTauriConfig.build?.beforeBuildCommand).toContain(
      "node scripts/prepare-windows-user-helper.mjs",
    );
  });

  it("launches only the fixed sibling helper through Explorer", () => {
    expect(layout).toContain(
      'USER_HELPER_EXECUTABLE_FILE_NAME: &str = "fyagent-user-helper.exe"',
    );
    expect(processLaunch).toContain("layout::USER_HELPER_EXECUTABLE_FILE_NAME");
    expect(processLaunch).toMatch(
      /current_exe\(\)[\s\S]+?\.parent\(\)[\s\S]+?\.join\(USER_HELPER_EXECUTABLE_FILE_NAME\)/u,
    );

    const helperLauncher = section(
      explorerLaunch,
      "    fn begin_fyagent_user_helper_launch(",
      "\n}\n\n/// Runs the COM automation call",
    );
    expect(helperLauncher).toContain("fixed_user_helper_path()");
    expect(helperLauncher).toContain(
      '"{INSTALL_ACTION} --job-id {job_id} --pipe {}"',
    );
    expect(helperLauncher).toContain("pipe_nonce.as_str()");
    expect(helperLauncher).toContain(
      "launch_path_from_explorer_with_arguments",
    );
    expect(helperLauncher).not.toMatch(
      /--(?:path|uri|root|operation|mode|port|scope)|runas/iu,
    );
  });

  it("keeps AddPackage only in the helper and ships no A2 runtime branch", () => {
    expect(runtime.match(/\.AddPackageByUriAsync\s*\(/gu)).toHaveLength(1);
    for (const source of [
      windowsAdapter,
      windowsDeployment,
      parentHelper,
      packageBridge,
    ]) {
      expect(source).not.toContain("AddPackageByUriAsync");
    }
    const activeWindowsInstaller = `${windowsAdapter}\n${windowsDeployment}\n${parentHelper}\n${packageBridge}\n${runtime}`;
    expect(activeWindowsInstaller).not.toMatch(
      /StagePackage(?:ByUri)?Async|RegisterPackagesByFullNameAsync|ProvisionPackage|RemoveForAllUsers/iu,
    );
    expect(windowsAdapter).toContain("install_dependencies.helper_runner.run(");
    expect(windowsAdapter).toContain("install_dependencies.deadlines");
  });

  it("treats Cancel as a request and exits only after true terminal observation", () => {
    expect(runtime.match(/\.Cancel\(\)/gu)).toHaveLength(1);
    const settle = section(
      runtime,
      "fn settle_after_failure",
      "\nfn wait_for_true_terminal",
    );
    expect(settle).toMatch(
      /terminal_status\(operation, completion\)\.is_none\(\)[\s\S]+?operation\.Cancel\(\)[\s\S]+?wait_for_true_terminal/u,
    );
    expect(runtime).toContain("operation.GetResults()");
    expect(runtime).toContain("operation.ErrorCode()");
    expect(runtime).toContain("operation.Close()");
    expect(runtime).not.toContain("E_ABORT");
    expect(helperLibrary).toMatch(/SETTLED_FAILURE_EXIT_CODE:\s*u8\s*=\s*10/u);
    expect(main).toContain("ExitCode::from(SETTLED_FAILURE_EXIT_CODE)");
    expect(parentHelper).not.toMatch(
      /GetExitCodeProcess|SETTLED_FAILURE_EXIT_CODE|TerminateProcess/u,
    );
  });
});
