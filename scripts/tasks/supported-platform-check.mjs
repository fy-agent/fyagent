#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath, pathToFileURL } from "node:url";
import { TextDecoder } from "node:util";
import { inflateSync } from "node:zlib";

export const ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

function isMain(importMetaUrl) {
  if (!process.argv[1]) return false;
  return pathToFileURL(path.resolve(process.argv[1])).href === importMetaUrl;
}

function fail(error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

const combine = (...parts) => parts.join("");
const whole = (value, flags = "iu") =>
  new RegExp(
    `(?:^|[^A-Za-z0-9_])${value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?:$|[^A-Za-z0-9_])`,
    flags,
  );
const contains = (value, flags = "iu") =>
  new RegExp(value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), flags);

export const SURFACE_MARKERS = Object.freeze({
  kernel: combine("lin", "ux"),
  subsystem: combine("w", "sl"),
  runnerFamily: combine("ubu", "ntu"),
  distributions: Object.freeze([
    combine("de", "bian"),
    combine("fe", "dora"),
    combine("cent", "os"),
    combine("rh", "el"),
    combine("red", " hat"),
    combine("open", "suse"),
    combine("al", "pine"),
    combine("nix", "os"),
  ]),
  imagePackage: combine("app", "image"),
  sandboxPackage: combine("flat", "pak"),
  sandboxCatalog: combine("flat", "hub"),
  archivePackage: combine("d", "eb"),
  nativePackage: combine("r", "pm"),
  displayToolkit: combine("g", "tk"),
  embeddedToolkit: combine("webkit2", "g", "tk"),
  windowProtocol: combine("x", "11"),
  compositorProtocol: combine("way", "land"),
  directoryConvention: combine("x", "dg"),
  objectFormat: combine("e", "lf"),
  serviceManager: combine("sys", "temd"),
  messageBus: combine("d", "bus"),
  packageCommands: Object.freeze([
    combine("a", "pt"),
    combine("a", "pt", "-get"),
    combine("d", "pkg"),
    combine("y", "um"),
    combine("d", "nf"),
    combine("pac", "man"),
    combine("zyp", "per"),
  ]),
  broadRustFamily: combine("un", "ix"),
  displayEnvironment: combine("DIS", "PLAY"),
  packageAddCommand: combine("a", "pk"),
  sandboxInstallCommand: combine("sn", "ap"),
});

const CONTENT_RULES = Object.freeze([
  {
    id: "retired-kernel",
    pattern: contains(SURFACE_MARKERS.kernel),
  },
  {
    id: "subsystem-bridge",
    pattern: contains(SURFACE_MARKERS.subsystem),
  },
  {
    id: "runner-family",
    pattern: contains(SURFACE_MARKERS.runnerFamily),
  },
  ...SURFACE_MARKERS.distributions.map((value) => ({
    id: "distribution-family",
    pattern: whole(value),
  })),
  {
    id: "image-package",
    pattern: contains(SURFACE_MARKERS.imagePackage),
  },
  {
    id: "sandbox-package",
    pattern: contains(SURFACE_MARKERS.sandboxPackage),
  },
  {
    id: "sandbox-catalog",
    pattern: contains(SURFACE_MARKERS.sandboxCatalog),
  },
  {
    id: "archive-package",
    pattern: whole(SURFACE_MARKERS.archivePackage),
  },
  {
    id: "native-package",
    pattern: whole(SURFACE_MARKERS.nativePackage),
  },
  {
    id: "display-toolkit",
    pattern: contains(SURFACE_MARKERS.displayToolkit),
  },
  {
    id: "embedded-display-toolkit",
    pattern: contains(SURFACE_MARKERS.embeddedToolkit),
  },
  {
    id: "window-protocol",
    pattern: whole(SURFACE_MARKERS.windowProtocol),
  },
  {
    id: "compositor-protocol",
    pattern: contains(SURFACE_MARKERS.compositorProtocol),
  },
  {
    id: "native-object-format",
    pattern: whole(SURFACE_MARKERS.objectFormat),
  },
  {
    id: "service-manager",
    pattern: whole(SURFACE_MARKERS.serviceManager),
  },
  {
    id: "message-bus",
    pattern: new RegExp(
      `(?:${SURFACE_MARKERS.messageBus}|${combine("d", "-bus")})`,
      "iu",
    ),
  },
  {
    id: "display-environment",
    pattern: new RegExp(
      `(?:["']${SURFACE_MARKERS.displayEnvironment}["']|(?:^|[^A-Za-z0-9_])${SURFACE_MARKERS.displayEnvironment}\\s*=)`,
      "u",
    ),
  },
  {
    id: "kernel-version-probe",
    pattern: /\/proc[\\/]version/iu,
  },
  {
    id: "host-release-probe",
    pattern: /\/etc[\\/]os-release/iu,
  },
  {
    id: "subsystem-mount-path",
    pattern: /\/mnt[\\/][A-Za-z](?:[\\/]|$)/u,
  },
  {
    id: "retired-home-layout",
    pattern: /\/home(?:[\\/]|$)/iu,
  },
  {
    id: "desktop-entry-shape",
    pattern: /\[Desktop Entry\]/iu,
  },
  {
    id: "open-bundle-target",
    pattern: /["']targets["']\s*:\s*["']all["']/iu,
  },
  {
    id: "negative-host-branch",
    pattern: /(?:process\.)?platform\s*!==?\s*["']win32["']/u,
  },
  {
    id: "reversed-negative-host-branch",
    pattern: /["']win32["']\s*!==?\s*(?:process\.)?platform/u,
  },
  {
    id: "negative-host-helper",
    pattern: /!\s*is(?:Windows|Mac(?:OS)?)\s*\(/u,
  },
  {
    id: "negative-host-helper",
    pattern:
      /(?:isWindows\s*\([^)]*\)\s*={2,3}\s*false|false\s*={2,3}\s*isWindows\s*\([^)]*\))/u,
  },
  ...SURFACE_MARKERS.packageCommands.map((value) => ({
    id: "package-command",
    pattern: whole(value),
  })),
  {
    id: "package-command",
    pattern: /(?:^|[^A-Za-z0-9_])apk\s+add(?:$|[^A-Za-z0-9_])/iu,
  },
  {
    id: "package-command",
    pattern: /(?:^|[^A-Za-z0-9_])snap\s+install(?:$|[^A-Za-z0-9_])/iu,
  },
]);

const PATH_RULES = Object.freeze(
  CONTENT_RULES.filter(
    ({ id }) =>
      ![
        "display-environment",
        "kernel-version-probe",
        "host-release-probe",
        "subsystem-mount-path",
        "retired-home-layout",
        "desktop-entry-shape",
        "open-bundle-target",
        "negative-host-branch",
        "reversed-negative-host-branch",
        "negative-host-helper",
        "package-command",
      ].includes(id),
  ),
);

const DIRECTORY_PATH_RULE = Object.freeze({
  id: "directory-convention",
  pattern: contains(SURFACE_MARKERS.directoryConvention),
});

const ARCHIVE_PREFIX = ".trellis/tasks/archive/";
export const GENERATED_STANDALONE_PREVIEW_PATH = "FyAgent-前端交互预览.html";
// The standalone preview is deterministic compiled output. Exclude only its
// generated body; its exact root filename is still inspected, while the V2
// source tree and build generator remain in the ordinary text scan.
const TEXT_EXCLUSIONS = new Set([
  "pnpm-lock.yaml",
  "src-tauri/Cargo.lock",
  GENERATED_STANDALONE_PREVIEW_PATH,
]);
export const ACTIVE_TASK_ENV = "FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK";
export const DEVELOPMENT_HOST_ADMISSION_PATHS = Object.freeze([
  "mise.lock",
  "mise.toml",
  "scripts/tasks/host-native.mjs",
  "scripts/tasks/lib.mjs",
  "scripts/tasks/lockfile-check.mjs",
  "scripts/tasks/system-check.mjs",
  "scripts/tasks/toolchain-check.mjs",
  "tests/developmentEnvironment.test.ts",
  "tests/localBuildBoundary.test.ts",
  "tests/miseTaskContract.test.ts",
  "tests/systemCheck.test.ts",
]);
const DEVELOPMENT_HOST_CONTENT_RULE_IDS = new Set([
  "retired-kernel",
  "native-object-format",
  "retired-home-layout",
]);
const UNSUPPORTED_CFG =
  '#[cfg(not(any(target_os = "windows", target_os = "macos")))]';
const TESTABLE_UNSUPPORTED_CFG =
  '#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]';

export const RUST_ALLOWANCE_CONTRACT = Object.freeze([
  Object.freeze({
    id: "runtime-path-import",
    file: "src-tauri/src/codex_desktop_runtime.rs",
    condition: UNSUPPORTED_CFG,
    next: "use std::path::Path;",
  }),
  Object.freeze({
    id: "runtime-adapter-import",
    file: "src-tauri/src/codex_desktop_runtime.rs",
    condition: UNSUPPORTED_CFG,
    next: "use crate::codex_desktop::{",
  }),
  Object.freeze({
    id: "runtime-probe-declaration",
    file: "src-tauri/src/codex_desktop_runtime.rs",
    condition: UNSUPPORTED_CFG,
    next: "#[derive(Debug, Default)]",
  }),
  Object.freeze({
    id: "runtime-probe-implementation",
    file: "src-tauri/src/codex_desktop_runtime.rs",
    condition: UNSUPPORTED_CFG,
    next: "impl DiskSpaceProbe for UnavailableDiskSpaceProbe {",
  }),
  Object.freeze({
    id: "runtime-dependency-rejection",
    file: "src-tauri/src/codex_desktop_runtime.rs",
    condition: UNSUPPORTED_CFG,
    next: "fn production_platform_dependencies() ->",
    nextPrefix: true,
  }),
  Object.freeze({
    id: "adapter-declaration",
    file: "src-tauri/src/codex_desktop/platform.rs",
    condition: TESTABLE_UNSUPPORTED_CFG,
    next: "#[derive(Debug, Clone)]",
  }),
  Object.freeze({
    id: "adapter-constructor",
    file: "src-tauri/src/codex_desktop/platform.rs",
    condition: TESTABLE_UNSUPPORTED_CFG,
    next: "impl UnsupportedPlatformAdapter {",
  }),
  Object.freeze({
    id: "adapter-implementation",
    file: "src-tauri/src/codex_desktop/platform.rs",
    condition: TESTABLE_UNSUPPORTED_CFG,
    next: "impl CodexDesktopPlatform for UnsupportedPlatformAdapter {",
  }),
]);
const RUST_CFG_MACRO_CONTRACT = Object.freeze(
  [
    [
      "src-tauri/src/lib.rs",
      'cfg!(target_os="macos")',
      1,
      ['focus_main_window: cfg!(target_os = "macos"),'],
    ],
    [
      "src-tauri/src/commands/misc.rs",
      'cfg!(target_os="windows")',
      1,
      [
        'Ok(lines.join(if cfg!(target_os = "windows") { "\\r\\n" } else { "\\n" }))',
      ],
    ],
    [
      "src-tauri/src/commands/misc.rs",
      'cfg!(target_os="macos")',
      1,
      [
        'fn fallback_user_shell() -> &\'static str { if cfg!(target_os = "macos") { "/bin/zsh" } else { "/bin/bash" } }',
      ],
    ],
    [
      "src-tauri/src/settings.rs",
      'cfg!(target_os="macos")',
      2,
      [
        'fn preferred_terminal_configuration() -> Option<PreferredTerminalConfiguration> { if cfg!(target_os = "macos") { Some(MACOS_PREFERRED_TERMINAL_CONFIGURATION) } else if cfg!(target_os = "windows") { Some(WINDOWS_PREFERRED_TERMINAL_CONFIGURATION) } else { None } }',
        'if cfg!(target_os = "macos") { assert!(crate::session_manager::terminal::is_supported_terminal_target(&effective)); }',
      ],
    ],
    [
      "src-tauri/src/settings.rs",
      'cfg!(target_os="windows")',
      1,
      [
        'fn preferred_terminal_configuration() -> Option<PreferredTerminalConfiguration> { if cfg!(target_os = "macos") { Some(MACOS_PREFERRED_TERMINAL_CONFIGURATION) } else if cfg!(target_os = "windows") { Some(WINDOWS_PREFERRED_TERMINAL_CONFIGURATION) } else { None } }',
      ],
    ],
    [
      "src-tauri/src/session_manager/terminal/mod.rs",
      '!cfg!(target_os="macos")',
      1,
      [
        'if !cfg!(target_os = "macos") { return Err("Terminal resume is only supported on macOS".to_string()); }',
      ],
    ],
    [
      "src-tauri/src/windows_runtime/mod.rs",
      'cfg!(all(target_os="windows",fyagent_windows_release))',
      1,
      [
        'pub(crate) const fn formal_windows_build() -> bool { cfg!(all(target_os = "windows", fyagent_windows_release)) }',
      ],
    ],
    [
      "src-tauri/src/codex_config.rs",
      'cfg!(target_os="windows")',
      1,
      [
        'if !codex_bundled_cli_allowed( cfg!(target_os = "windows"), crate::windows_runtime::formal_windows_build(), ) { return Ok(None); }',
      ],
    ],
  ].map(([file, expression, count, anchors]) =>
    Object.freeze({ file, expression, count, anchors: Object.freeze(anchors) }),
  ),
);
const RUST_ARCH_CFG_CONTRACT = Object.freeze(
  [
    [
      "src-tauri/src/codex_desktop_runtime.rs",
      '#[cfg(target_arch="x86_64")]',
      2,
      [
        'const fn current_windows_architecture() -> CpuArchitecture { #[cfg(target_arch = "x86_64")] { CpuArchitecture::X86_64 } #[cfg(target_arch = "aarch64")] { CpuArchitecture::Aarch64 } #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))] { CpuArchitecture::Unsupported } }',
        'const fn current_macos_architecture() -> CpuArchitecture { #[cfg(target_arch = "aarch64")] { CpuArchitecture::Aarch64 } #[cfg(target_arch = "x86_64")] { CpuArchitecture::X86_64UnsupportedMac } #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))] { CpuArchitecture::Unsupported } }',
      ],
    ],
    [
      "src-tauri/src/codex_desktop_runtime.rs",
      '#[cfg(target_arch="aarch64")]',
      2,
      [
        'const fn current_windows_architecture() -> CpuArchitecture { #[cfg(target_arch = "x86_64")] { CpuArchitecture::X86_64 } #[cfg(target_arch = "aarch64")] { CpuArchitecture::Aarch64 } #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))] { CpuArchitecture::Unsupported } }',
        'const fn current_macos_architecture() -> CpuArchitecture { #[cfg(target_arch = "aarch64")] { CpuArchitecture::Aarch64 } #[cfg(target_arch = "x86_64")] { CpuArchitecture::X86_64UnsupportedMac } #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))] { CpuArchitecture::Unsupported } }',
      ],
    ],
    [
      "src-tauri/src/codex_desktop_runtime.rs",
      '#[cfg(not(any(target_arch="x86_64",target_arch="aarch64")))]',
      1,
      [
        'const fn current_windows_architecture() -> CpuArchitecture { #[cfg(target_arch = "x86_64")] { CpuArchitecture::X86_64 } #[cfg(target_arch = "aarch64")] { CpuArchitecture::Aarch64 } #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))] { CpuArchitecture::Unsupported } }',
      ],
    ],
    [
      "src-tauri/src/codex_desktop_runtime.rs",
      '#[cfg(not(any(target_arch="aarch64",target_arch="x86_64")))]',
      1,
      [
        'const fn current_macos_architecture() -> CpuArchitecture { #[cfg(target_arch = "aarch64")] { CpuArchitecture::Aarch64 } #[cfg(target_arch = "x86_64")] { CpuArchitecture::X86_64UnsupportedMac } #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))] { CpuArchitecture::Unsupported } }',
      ],
    ],
  ].map(([file, expression, count, anchors]) =>
    Object.freeze({ file, expression, count, anchors: Object.freeze(anchors) }),
  ),
);
const MANUAL_TARGET_MARKER = combine("CARGO_CFG_TARGET_", "OS");
export const RUST_MANUAL_TARGET_CONTRACT = Object.freeze([
  Object.freeze({
    file: "src-tauri/build.rs",
    count: 1,
    snippet: `
      let target_os = std::env::var("${MANUAL_TARGET_MARKER}").unwrap_or_default();
      match target_os.as_str() {
        "macos" => { tauri_build::build(); return; }
        "windows" => {}
        _ => { tauri_build::build(); return; }
      }
    `,
  }),
  Object.freeze({
    file: "src-tauri/user-helper/build.rs",
    count: 1,
    snippet: `
      if std::env::var("${MANUAL_TARGET_MARKER}").as_deref() == Ok("windows")
        && std::env::var_os("CARGO_FEATURE_HELPER_RUNTIME").is_some()
      {
        embed_resource::compile(
          "windows/fyagent-user-helper.rc",
          embed_resource::ParamsIncludeDirs(&["windows"]),
        )
        .manifest_required()
        .expect("failed to embed the fyagent-user-helper asInvoker manifest");
      }
    `,
  }),
]);
const RUST_STD_OS_CONTRACT = Object.freeze({
  file: "src-tauri/src/panic_hook.rs",
  count: 1,
  anchor:
    "let os = std::env::consts::OS; let arch = std::env::consts::ARCH; let family = std::env::consts::FAMILY;",
});

const DATA_HOME_VARIABLE = `${SURFACE_MARKERS.directoryConvention.toUpperCase()}_DATA_HOME`;
const BIN_DIRECTORY_VARIABLE = `${SURFACE_MARKERS.directoryConvention.toUpperCase()}_BIN_DIR`;
const DATA_HOME_IDENTIFIER = combine("OPENCODE_DATA_", "HOME_ENV");
export const MACOS_POSIX_CONTRACT = Object.freeze([
  Object.freeze({
    id: "data-home-declaration",
    file: "src-tauri/src/opencode_config.rs",
    snippet: `#[cfg(any(target_os = "macos", test))]\npub(crate) const ${DATA_HOME_IDENTIFIER}: &str = "${DATA_HOME_VARIABLE}";`,
  }),
  Object.freeze({
    id: "data-home-macos-read",
    file: "src-tauri/src/opencode_config.rs",
    snippet: `#[cfg(target_os = "macos")]\npub(crate) fn get_opencode_data_dir() -> PathBuf {\n    resolve_opencode_data_dir(\n        &crate::config::get_home_dir(),\n        std::env::var_os(${DATA_HOME_IDENTIFIER}).as_deref(),`,
  }),
  Object.freeze({
    id: "data-home-windows-ignore",
    file: "src-tauri/src/opencode_config.rs",
    snippet:
      '#[cfg(target_os = "windows")]\npub(crate) fn get_opencode_data_dir() -> PathBuf {\n    resolve_opencode_data_dir(&crate::config::get_home_dir(), None)',
  }),
  Object.freeze({
    id: "session-data-resolver",
    file: "src-tauri/src/session_manager/providers/opencode.rs",
    snippet: "crate::opencode_config::get_opencode_data_dir()",
  }),
  Object.freeze({
    id: "session-scan-db-resolver",
    file: "src-tauri/src/session_manager/providers/opencode.rs",
    snippet: "let db_path = crate::opencode_config::get_opencode_db_path();",
  }),
  Object.freeze({
    id: "session-delete-db-resolver",
    file: "src-tauri/src/session_manager/providers/opencode.rs",
    snippet: "&crate::opencode_config::get_opencode_db_path(),",
  }),
  Object.freeze({
    id: "usage-db-resolver",
    file: "src-tauri/src/services/session_usage_opencode.rs",
    snippet: "use crate::opencode_config::get_opencode_db_path;",
  }),
  Object.freeze({
    id: "cli-bin-macos-read",
    file: "src-tauri/src/commands/misc.rs",
    snippet: `#[cfg(target_os = "macos")]\n        let ambient_paths = (\n            std::env::var_os("OPENCODE_INSTALL_DIR"),\n            std::env::var_os("${BIN_DIRECTORY_VARIABLE}"),\n            std::env::var_os("GOPATH"),\n        );`,
  }),
  Object.freeze({
    id: "cli-bin-windows-ignore",
    file: "src-tauri/src/commands/misc.rs",
    snippet:
      '#[cfg(target_os = "windows")]\n        let ambient_paths = (None, None, None);',
  }),
]);

const DIRECTORY_IDENTIFIER = SURFACE_MARKERS.directoryConvention;
const DIRECTORY_MARKER_PATTERN = new RegExp(DIRECTORY_IDENTIFIER, "iu");
const DIRECTORY_OCCURRENCE_CONTRACT = Object.freeze(
  [
    [
      "src-tauri/src/opencode_config.rs",
      `pub(crate) const ${DATA_HOME_IDENTIFIER}: &str = "${DATA_HOME_VARIABLE}";`,
      `#[cfg(any(target_os = "macos", test))] pub(crate) const ${DATA_HOME_IDENTIFIER}: &str = "${DATA_HOME_VARIABLE}";`,
    ],
    [
      "src-tauri/src/opencode_config.rs",
      `/// macOS 优先级: OPENCODE_DB 环境变量 > ${DATA_HOME_VARIABLE} > ~/.local/share/opencode/opencode.db。`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `///   $OPENCODE_INSTALL_DIR > $${BIN_DIRECTORY_VARIABLE} > $HOME/bin > $HOME/.opencode/bin`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `${DIRECTORY_IDENTIFIER}_bin_dir: Option<std::ffi::OsString>,`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `push_env_single_dir(&mut paths, ${DIRECTORY_IDENTIFIER}_bin_dir);`,
      `fn opencode_extra_search_paths( home: &Path, opencode_install_dir: Option<std::ffi::OsString>, ${DIRECTORY_IDENTIFIER}_bin_dir: Option<std::ffi::OsString>, gopath: Option<std::ffi::OsString>, ) -> Vec<std::path::PathBuf> { let mut paths = Vec::new(); push_env_single_dir(&mut paths, opencode_install_dir); push_env_single_dir(&mut paths, ${DIRECTORY_IDENTIFIER}_bin_dir);`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `std::env::var_os("${BIN_DIRECTORY_VARIABLE}"),`,
      `#[cfg(target_os = "macos")] let ambient_paths = ( std::env::var_os("OPENCODE_INSTALL_DIR"), std::env::var_os("${BIN_DIRECTORY_VARIABLE}"), std::env::var_os("GOPATH"), );`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `let ${DIRECTORY_IDENTIFIER}_bin_dir = Some(std::ffi::OsString::from("/custom/${DIRECTORY_IDENTIFIER}/bin"));`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `let paths = opencode_extra_search_paths(&home, install_dir, ${DIRECTORY_IDENTIFIER}_bin_dir, gopath);`,
    ],
    [
      "src-tauri/src/commands/misc.rs",
      `assert_eq!(paths[1], PathBuf::from("/custom/${DIRECTORY_IDENTIFIER}/bin"));`,
    ],
  ].map(([file, snippet, anchor]) =>
    Object.freeze({ file, snippet, anchor, count: 1 }),
  ),
);
const DATA_HOME_REFERENCE_CONTRACT = Object.freeze(
  [
    [
      "src-tauri/src/opencode_config.rs",
      `pub(crate) const ${DATA_HOME_IDENTIFIER}: &str = "${DATA_HOME_VARIABLE}";`,
    ],
    [
      "src-tauri/src/opencode_config.rs",
      `std::env::var_os(${DATA_HOME_IDENTIFIER}).as_deref(),`,
      `#[cfg(target_os = "macos")] pub(crate) fn get_opencode_data_dir() -> PathBuf { resolve_opencode_data_dir( &crate::config::get_home_dir(), std::env::var_os(${DATA_HOME_IDENTIFIER}).as_deref(), ) }`,
    ],
    [
      "src-tauri/src/opencode_config.rs",
      `let _data_home_guard = EnvVarGuard::remove(${DATA_HOME_IDENTIFIER});`,
    ],
    [
      "src-tauri/src/opencode_config.rs",
      `let _guard = EnvVarGuard::set(${DATA_HOME_IDENTIFIER}, override_root.as_os_str());`,
    ],
    [
      "src-tauri/src/opencode_config.rs",
      `let _ambient_guard = EnvVarGuard::set(${DATA_HOME_IDENTIFIER}, ambient_root.as_os_str());`,
    ],
    [
      "tests/codexWindowsUserScopeContract.test.ts",
      combine(
        "`const ",
        DATA_HOME_IDENTIFIER,
        ': &str = "${macosDataHomeVariable}";`,',
      ),
    ],
    [
      "tests/codexWindowsUserScopeContract.test.ts",
      combine(
        '/#\\[cfg\\(target_os = "macos"\\)\\]\\s+pub\\(crate\\) fn get_opencode_data_dir\\(\\) -> PathBuf \\{[\\s\\S]*?std::env::var_os\\(',
        DATA_HOME_IDENTIFIER,
        "\\)/,",
      ),
    ],
  ].map(([file, snippet, anchor]) =>
    Object.freeze({ file, snippet, anchor, count: 1 }),
  ),
);
function normalizeRepositoryPath(value) {
  if (
    typeof value !== "string" ||
    value === "" ||
    value.includes("\0") ||
    value.includes("\\") ||
    path.posix.isAbsolute(value)
  ) {
    throw new Error(`Invalid repository path: ${String(value)}`);
  }
  const normalized = path.posix.normalize(value);
  if (
    normalized !== value ||
    normalized === "." ||
    normalized === ".." ||
    normalized.startsWith("../")
  ) {
    throw new Error(`Non-canonical repository path: ${value}`);
  }
  return value;
}

function isArchivePath(relativePath) {
  return relativePath.startsWith(ARCHIVE_PREFIX);
}

function isDotPath(relativePath) {
  // Dot-prefixed top-level entries (`.trellis/`, `.codebuddy/`, `.codex/`,
  // `.agents/`, and dotfiles such as `.gitignore`) hold workspace, agent, and
  // template configuration rather than shipped platform surface. Excluding
  // them keeps template edits from forcing contract churn.
  return relativePath.startsWith(".");
}

function activeTaskIdFromPath(value) {
  normalizeRepositoryPath(value);
  const match =
    /^\.trellis\/tasks\/(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])-([a-z0-9]+(?:-[a-z0-9]+)*)$/u.exec(
      value,
    );
  if (!match) {
    throw new Error(
      "The temporary exclusion must be a canonical .trellis/tasks/MM-DD-<id> direct child",
    );
  }
  return match[1];
}

export function validateActiveTaskExclusion(
  value,
  {
    root = ROOT,
    io = fs,
    sessionResolver = resolveAuthoritativeActiveTask,
    runner = spawnSync,
  } = {},
) {
  const activeTaskId = activeTaskIdFromPath(value);

  const taskRoot = path.join(root, ".trellis", "tasks");
  const taskDirectory = path.join(root, ...value.split("/"));
  const taskStat = io.lstatSync(taskDirectory);
  if (!taskStat.isDirectory() || taskStat.isSymbolicLink()) {
    throw new Error("The temporary exclusion must name a real task directory");
  }

  const realTaskRoot = io.realpathSync(taskRoot);
  const realTaskDirectory = io.realpathSync(taskDirectory);
  const relative = path.relative(realTaskRoot, realTaskDirectory);
  if (
    relative === "" ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative) ||
    relative.split(path.sep).length !== 1
  ) {
    throw new Error("The temporary exclusion escaped the active task root");
  }

  const metadataPath = path.join(taskDirectory, "task.json");
  const metadataStat = io.lstatSync(metadataPath);
  if (!metadataStat.isFile() || metadataStat.isSymbolicLink()) {
    throw new Error("The temporary exclusion has no regular task metadata");
  }
  const metadata = JSON.parse(io.readFileSync(metadataPath, "utf8"));
  if (
    metadata.id !== activeTaskId ||
    metadata.name !== activeTaskId ||
    metadata.status !== "in_progress"
  ) {
    throw new Error(
      "The temporary exclusion metadata does not match its canonical in-progress task path",
    );
  }
  const authoritative = sessionResolver(root, runner);
  if (authoritative !== value) {
    throw new Error(
      "The temporary exclusion does not match the current session task",
    );
  }
  return value;
}

export function resolveAuthoritativeActiveTask(
  root = ROOT,
  runner = spawnSync,
) {
  const result = runner(
    "python",
    [".trellis/scripts/task.py", "current", "--source", "--json"],
    { cwd: root, encoding: "utf8", windowsHide: true },
  );
  if (result.error) throw result.error;
  if (result.status !== 0 || typeof result.stdout !== "string") {
    throw new Error("The current session has no active-task pointer");
  }
  let payload;
  try {
    payload = JSON.parse(result.stdout);
  } catch {
    throw new Error("The active-task command returned invalid JSON");
  }
  if (
    payload === null ||
    typeof payload !== "object" ||
    Array.isArray(payload) ||
    payload.stale !== false ||
    payload.current_task === null ||
    typeof payload.current_task !== "object" ||
    Array.isArray(payload.current_task) ||
    typeof payload.current_task.dir !== "string" ||
    typeof payload.source !== "string" ||
    !/^session:[A-Za-z0-9._-]+$/u.test(payload.source)
  ) {
    throw new Error(
      "The temporary exclusion is not directly active in the current session",
    );
  }
  return payload.current_task.dir;
}

export function parseArguments(argv, environment = process.env) {
  let direct;
  if (argv.length > 0) {
    if (argv.length !== 2 || argv[0] !== "--exclude-active-task") {
      throw new Error(
        "Usage: supported-platform-check.mjs [--exclude-active-task <path>]",
      );
    }
    direct = argv[1];
  }

  const optionalEnvironmentValue = (name) => {
    if (!Object.hasOwn(environment, name)) return undefined;
    const value = environment[name];
    if (typeof value !== "string" || value === "") {
      throw new Error(`${name} must be a non-empty string when provided`);
    }
    return value;
  };
  const fromTask = optionalEnvironmentValue("usage_exclude_active_task");
  const fromLeaf = optionalEnvironmentValue(ACTIVE_TASK_ENV);
  const provided = [direct, fromTask, fromLeaf].filter(
    (value) => value !== undefined,
  );
  if (provided.length > 1) {
    throw new Error(
      "The temporary exclusion was provided through multiple inputs",
    );
  }
  return direct ?? fromTask ?? fromLeaf;
}

export function isExcludedPath(relativePath, activeTask) {
  normalizeRepositoryPath(relativePath);
  return (
    isDotPath(relativePath) ||
    isArchivePath(relativePath) ||
    (activeTask !== undefined &&
      (relativePath === activeTask ||
        relativePath.startsWith(`${activeTask}/`)))
  );
}

export function isTextExcludedPath(relativePath) {
  normalizeRepositoryPath(relativePath);
  return TEXT_EXCLUSIONS.has(relativePath);
}

function stripOpaqueSvgPayload(source) {
  return source.replace(
    /(data:[^"']*?;base64,)[A-Za-z0-9+/=\r\n]+/giu,
    "$1[payload]",
  );
}

const OPAQUE_BINARY_EXTENSIONS = new Set([
  ".icns",
  ".ico",
  ".jpg",
  ".png",
  ".webp",
]);
const RASTER_ASSET_SCHEMA = "fyagent-supported-platform-raster-baseline/v1";
const RASTER_MANIFEST_RELATIVE_PATH =
  "scripts/tasks/supported-platform-raster-assets.json";
const RASTER_MANIFEST_PATH = path.join(
  ROOT,
  ...RASTER_MANIFEST_RELATIVE_PATH.split("/"),
);
export function loadRasterAssetManifest(
  manifestPath = RASTER_MANIFEST_PATH,
  io = fs,
) {
  const manifestStat = io.lstatSync(manifestPath);
  if (!manifestStat.isFile() || manifestStat.isSymbolicLink()) {
    throw new Error(
      "Supported-platform raster asset manifest must be a regular non-symlink file",
    );
  }
  const manifest = JSON.parse(io.readFileSync(manifestPath, "utf8"));
  if (
    manifest === null ||
    typeof manifest !== "object" ||
    Array.isArray(manifest) ||
    Object.keys(manifest).sort().join("\0") !== "assets\0schema" ||
    manifest.schema !== RASTER_ASSET_SCHEMA ||
    !Array.isArray(manifest.assets)
  ) {
    throw new Error("Invalid supported-platform raster asset manifest schema");
  }
  const assets = manifest.assets.map((record, index) => {
    if (
      !Array.isArray(record) ||
      record.length !== 2 ||
      typeof record[0] !== "string" ||
      typeof record[1] !== "string"
    ) {
      throw new Error(`Invalid raster asset manifest record ${index}`);
    }
    const relativePath = normalizeRepositoryPath(record[0]);
    const digest = record[1];
    if (
      !OPAQUE_BINARY_EXTENSIONS.has(
        path.posix.extname(relativePath).toLowerCase(),
      ) ||
      !/^[0-9a-f]{64}$/u.test(digest)
    ) {
      throw new Error(`Invalid raster asset manifest entry: ${relativePath}`);
    }
    return Object.freeze({ path: relativePath, digest });
  });
  for (let index = 0; index < assets.length; index += 1) {
    const current = assets[index];
    const previous = assets[index - 1];
    if (previous && previous.path.localeCompare(current.path, "en") >= 0) {
      throw new Error(
        `Raster asset manifest is not uniquely sorted: ${current.path}`,
      );
    }
  }
  return Object.freeze(assets);
}
export const RASTER_ASSET_CONTRACT = loadRasterAssetManifest();
const RASTER_ASSET_DIGESTS = new Map(
  RASTER_ASSET_CONTRACT.map((asset) => [asset.path, asset.digest]),
);

const STRUCTURE_ASSET_SCHEMA =
  "fyagent-supported-platform-structure-baseline/v1";
const STRUCTURE_MANIFEST_RELATIVE_PATH =
  "scripts/tasks/supported-platform-structure-assets.json";
const STRUCTURE_MANIFEST_PATH = path.join(
  ROOT,
  ...STRUCTURE_MANIFEST_RELATIVE_PATH.split("/"),
);
const STRUCTURE_SOURCE_EXTENSIONS = new Set([
  ".cjs",
  ".cts",
  ".js",
  ".json",
  ".jsonc",
  ".jsx",
  ".mjs",
  ".mts",
  ".ps1",
  ".rs",
  ".sh",
  ".toml",
  ".ts",
  ".tsx",
  ".yaml",
  ".yml",
]);
const STRUCTURE_ASSET_EXCLUSIONS = new Set([
  ...TEXT_EXCLUSIONS,
  RASTER_MANIFEST_RELATIVE_PATH,
  STRUCTURE_MANIFEST_RELATIVE_PATH,
]);
const PLATFORM_STRUCTURE_PATTERN = new RegExp(
  [
    "\\btarget_(?:os|family|arch|vendor|env|feature)\\b",
    "\\bCARGO_CFG_TARGET_",
    "std\\s*::\\s*env\\s*::\\s*consts\\s*::\\s*(?:OS|FAMILY)\\b",
    "std\\s*::\\s*env\\s*::\\s*(?:var|var_os)\\s*\\(\\s*[\"'](?:TARGET|HOST|FAMILY)[\"']",
    "process\\s*(?:(?:\\?\\.|\\.)\\s*platform|(?:\\?\\.)?\\s*\\[\\s*[\"']platform[\"']\\s*\\])",
    "\\bis(?:Windows|Mac|MacOS)\\s*(?:\\?\\.)?\\s*\\(",
    SURFACE_MARKERS.directoryConvention,
    "\\bcfg!?\\s*\\([^)]*\\b(?:windows|macos)\\b",
  ].join("|"),
  "iu",
);

export function loadStructureAssetManifest(
  manifestPath = STRUCTURE_MANIFEST_PATH,
  io = fs,
) {
  const stat = io.lstatSync(manifestPath);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(
      "Supported-platform structure manifest must be a regular non-symlink file",
    );
  }
  const manifest = JSON.parse(io.readFileSync(manifestPath, "utf8"));
  if (
    manifest === null ||
    typeof manifest !== "object" ||
    Array.isArray(manifest) ||
    Object.keys(manifest).sort().join("\0") !== "assets\0schema" ||
    manifest.schema !== STRUCTURE_ASSET_SCHEMA ||
    !Array.isArray(manifest.assets)
  ) {
    throw new Error("Invalid supported-platform structure manifest schema");
  }
  const assets = manifest.assets.map((record, index) => {
    if (
      !Array.isArray(record) ||
      record.length !== 2 ||
      typeof record[0] !== "string" ||
      typeof record[1] !== "string"
    ) {
      throw new Error(`Invalid structure manifest record ${index}`);
    }
    const relativePath = normalizeRepositoryPath(record[0]);
    if (!/^[0-9a-f]{64}$/u.test(record[1])) {
      throw new Error(`Invalid structure manifest digest: ${relativePath}`);
    }
    return Object.freeze({ path: relativePath, digest: record[1] });
  });
  for (let index = 0; index < assets.length; index += 1) {
    const current = assets[index];
    const previous = assets[index - 1];
    if (previous && previous.path.localeCompare(current.path, "en") >= 0) {
      throw new Error(
        `Structure manifest is not uniquely sorted: ${current.path}`,
      );
    }
  }
  return Object.freeze(assets);
}

export const STRUCTURE_ASSET_CONTRACT = loadStructureAssetManifest();
const STRUCTURE_ASSET_DIGESTS = new Map(
  STRUCTURE_ASSET_CONTRACT.map((asset) => [asset.path, asset.digest]),
);

function structureSourceCandidate(relativePath, source) {
  const basename = path.posix.basename(relativePath);
  if (basename === "Cargo.toml" || basename === "build.rs") return true;
  return PLATFORM_STRUCTURE_PATTERN.test(source);
}

export function validateStructureAssetInventory(
  currentPaths,
  indexModes,
  { root = ROOT, io = fs, activeTask } = {},
) {
  const candidates = [];
  const buffers = new Map();
  for (const relativePath of currentPaths) {
    if (
      isExcludedPath(relativePath, activeTask) ||
      STRUCTURE_ASSET_EXCLUSIONS.has(relativePath) ||
      !STRUCTURE_SOURCE_EXTENSIONS.has(
        path.posix.extname(relativePath).toLowerCase(),
      )
    ) {
      continue;
    }
    const absolute = path.join(root, ...relativePath.split("/"));
    let stat;
    try {
      stat = io.lstatSync(absolute);
    } catch (error) {
      // `git ls-files --cached` includes tracked deletions until the caller
      // stages them. Treat an absent working-tree path as deleted; a deleted
      // sealed candidate still fails below when the candidate inventory no
      // longer matches the reviewed manifest.
      if (error && typeof error === "object" && error.code === "ENOENT") {
        continue;
      }
      throw error;
    }
    if (!stat.isFile() || stat.isSymbolicLink()) {
      throw new Error(
        `Executable structure source must be a regular file: ${relativePath}`,
      );
    }
    const buffer = io.readFileSync(absolute);
    const source = new TextDecoder("utf-8", { fatal: true }).decode(buffer);
    if (!structureSourceCandidate(relativePath, source)) continue;
    candidates.push(relativePath);
    buffers.set(relativePath, buffer);
  }
  candidates.sort((left, right) => left.localeCompare(right, "en"));
  const expected = STRUCTURE_ASSET_CONTRACT.map((asset) => asset.path);
  if (
    candidates.length !== expected.length ||
    candidates.some((candidate, index) => candidate !== expected[index])
  ) {
    throw new Error("Supported-platform structure candidate inventory drifted");
  }
  for (const relativePath of candidates) {
    if (indexModes.get(relativePath) !== "100644") {
      throw new Error(
        `Structure asset must have Git index mode 100644: ${relativePath}`,
      );
    }
    const digest = createHash("sha256")
      .update(buffers.get(relativePath))
      .digest("hex");
    if (STRUCTURE_ASSET_DIGESTS.get(relativePath) !== digest) {
      throw new Error(
        `Supported-platform structure identity drifted: ${relativePath}`,
      );
    }
  }
  return candidates;
}
const MAX_IMAGE_METADATA_BYTES = 1024 * 1024;
const MAX_IMAGE_METADATA_SOURCE_BYTES = 1024 * 1024;

function createImageMetadataBudget(relativePath) {
  let sourceBytes = 0;
  let decodedBytes = 0;
  const metadata = [];
  return {
    reserveSource(length) {
      sourceBytes += length;
      if (sourceBytes > MAX_IMAGE_METADATA_SOURCE_BYTES) {
        throw new Error(`Image metadata budget exceeded: ${relativePath}`);
      }
    },
    addDecoded(value) {
      const text = value || "";
      decodedBytes += Buffer.byteLength(text, "utf8");
      if (decodedBytes > MAX_IMAGE_METADATA_BYTES) {
        throw new Error(`Image metadata budget exceeded: ${relativePath}`);
      }
      if (text) metadata.push(text);
    },
    remainingDecodedBytes() {
      return MAX_IMAGE_METADATA_BYTES - decodedBytes;
    },
    finish() {
      return metadata.join("\n");
    },
  };
}

function inflateMetadata(payload, budget, relativePath) {
  const remaining = budget.remainingDecodedBytes();
  if (remaining <= 0) {
    throw new Error(`Image metadata budget exceeded: ${relativePath}`);
  }
  try {
    return inflateSync(payload, { maxOutputLength: remaining }).toString(
      "utf8",
    );
  } catch (error) {
    throw new Error(
      `Image metadata budget or compression invalid: ${relativePath}`,
      {
        cause: error,
      },
    );
  }
}

const PNG_CRC_TABLE = Uint32Array.from({ length: 256 }, (_, value) => {
  let crc = value;
  for (let bit = 0; bit < 8; bit += 1) {
    crc = (crc & 1) !== 0 ? 0xedb88320 ^ (crc >>> 1) : crc >>> 1;
  }
  return crc >>> 0;
});

function pngCrc32(buffer) {
  let crc = 0xffffffff;
  for (const value of buffer) {
    crc = PNG_CRC_TABLE[(crc ^ value) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function printableMetadata(buffer) {
  return (
    buffer
      .toString("latin1")
      .match(/[\t\x20-\x7e]{3,}/gu)
      ?.join("\n") ?? ""
  );
}

function parsePng(
  buffer,
  relativePath,
  budget = createImageMetadataBudget(relativePath),
) {
  let offset = 8;
  let sawHeader = false;
  let sawPixels = false;
  let sawEnd = false;
  while (offset < buffer.length) {
    if (offset + 12 > buffer.length) {
      throw new Error(`Invalid PNG chunk boundary: ${relativePath}`);
    }
    const length = buffer.readUInt32BE(offset);
    const type = buffer.subarray(offset + 4, offset + 8).toString("ascii");
    const payloadStart = offset + 8;
    const payloadEnd = payloadStart + length;
    const chunkEnd = payloadEnd + 4;
    if (!/^[A-Za-z]{4}$/u.test(type) || chunkEnd > buffer.length) {
      throw new Error(`Invalid PNG chunk: ${relativePath}`);
    }
    const payload = buffer.subarray(payloadStart, payloadEnd);
    const expectedCrc = buffer.readUInt32BE(payloadEnd);
    const actualCrc = pngCrc32(buffer.subarray(offset + 4, payloadEnd));
    if (actualCrc !== expectedCrc) {
      throw new Error(`Invalid PNG chunk checksum: ${relativePath}`);
    }
    if (!sawHeader) {
      if (type !== "IHDR" || length !== 13) {
        throw new Error(`PNG must start with IHDR: ${relativePath}`);
      }
      sawHeader = true;
    } else if (type === "IHDR") {
      throw new Error(`PNG contains duplicate IHDR: ${relativePath}`);
    }
    if (type === "IDAT") sawPixels = true;
    if (type === "tEXt") {
      budget.reserveSource(payload.length);
      budget.addDecoded(payload.toString("latin1"));
    }
    if (type === "zTXt") {
      const separator = payload.indexOf(0);
      if (separator < 0 || payload[separator + 1] !== 0) {
        throw new Error(`Invalid PNG zTXt metadata: ${relativePath}`);
      }
      budget.reserveSource(payload.length);
      budget.addDecoded(payload.subarray(0, separator).toString("latin1"));
      const compressed = payload.subarray(separator + 2);
      budget.addDecoded(inflateMetadata(compressed, budget, relativePath));
    }
    if (type === "iTXt") {
      const first = payload.indexOf(0);
      if (
        first < 0 ||
        first + 3 > payload.length ||
        ![0, 1].includes(payload[first + 1]) ||
        payload[first + 2] !== 0
      ) {
        throw new Error(`Invalid PNG iTXt metadata: ${relativePath}`);
      }
      let cursor = first + 3;
      const languageEnd = payload.indexOf(0, cursor);
      if (languageEnd < 0)
        throw new Error(`Invalid PNG iTXt metadata: ${relativePath}`);
      cursor = languageEnd + 1;
      const translatedEnd = payload.indexOf(0, cursor);
      if (translatedEnd < 0)
        throw new Error(`Invalid PNG iTXt metadata: ${relativePath}`);
      const text = payload.subarray(translatedEnd + 1);
      budget.reserveSource(payload.length);
      budget.addDecoded(payload.subarray(0, first).toString("utf8"));
      if (payload[first + 1] === 1) {
        budget.addDecoded(inflateMetadata(text, budget, relativePath));
      } else {
        budget.addDecoded(text.toString("utf8"));
      }
    }
    if (type === "iCCP") {
      const separator = payload.indexOf(0);
      if (separator < 0 || payload[separator + 1] !== 0) {
        throw new Error(`Invalid PNG iCCP metadata: ${relativePath}`);
      }
      budget.reserveSource(payload.length);
      budget.addDecoded(payload.subarray(0, separator).toString("latin1"));
      const compressed = payload.subarray(separator + 2);
      budget.addDecoded(inflateMetadata(compressed, budget, relativePath));
    } else if (type === "eXIf") {
      budget.reserveSource(payload.length);
      budget.addDecoded(printableMetadata(payload));
    } else if (
      !["IHDR", "PLTE", "IDAT", "IEND", "tEXt", "zTXt", "iTXt"].includes(type)
    ) {
      if (type[0] === type[0].toUpperCase()) {
        throw new Error(`Unknown critical PNG chunk ${type}: ${relativePath}`);
      }
      // Ancillary chunks are not implicitly opaque: printable metadata must
      // still pass through the normal platform-surface scan.
      budget.reserveSource(payload.length);
      budget.addDecoded(printableMetadata(payload));
    }
    offset = chunkEnd;
    if (type === "IEND") {
      if (length !== 0 || offset !== buffer.length) {
        throw new Error(`PNG has trailing container data: ${relativePath}`);
      }
      sawEnd = true;
      break;
    }
  }
  if (!sawHeader || !sawPixels || !sawEnd) {
    throw new Error(`Incomplete PNG image container: ${relativePath}`);
  }
  return budget.finish();
}

function parseJpeg(buffer, relativePath) {
  if (
    buffer.length < 4 ||
    buffer[buffer.length - 2] !== 0xff ||
    buffer[buffer.length - 1] !== 0xd9
  ) {
    throw new Error(`JPEG has no exact EOI boundary: ${relativePath}`);
  }
  let offset = 2;
  let sawScan = false;
  const budget = createImageMetadataBudget(relativePath);
  while (offset < buffer.length - 2) {
    if (buffer[offset] !== 0xff) {
      if (!sawScan) throw new Error(`Invalid JPEG marker: ${relativePath}`);
      offset += 1;
      continue;
    }
    while (buffer[offset] === 0xff) offset += 1;
    const marker = buffer[offset++];
    if (marker === 0x00 || (marker >= 0xd0 && marker <= 0xd7)) continue;
    if (marker === 0xd9) {
      if (offset !== buffer.length)
        throw new Error(`JPEG has trailing data: ${relativePath}`);
      break;
    }
    if (offset + 2 > buffer.length)
      throw new Error(`Invalid JPEG segment: ${relativePath}`);
    const length = buffer.readUInt16BE(offset);
    if (length < 2 || offset + length > buffer.length) {
      throw new Error(`Invalid JPEG segment length: ${relativePath}`);
    }
    const payload = buffer.subarray(offset + 2, offset + length);
    if ((marker >= 0xe0 && marker <= 0xef) || marker === 0xfe) {
      budget.reserveSource(payload.length);
      budget.addDecoded(printableMetadata(payload));
    }
    offset += length;
    if (marker === 0xda) sawScan = true;
  }
  if (!sawScan) throw new Error(`JPEG contains no image scan: ${relativePath}`);
  return budget.finish();
}

function parseWebp(buffer, relativePath) {
  if (buffer.length < 12 || buffer.readUInt32LE(4) + 8 !== buffer.length) {
    throw new Error(
      `WebP RIFF length does not match container: ${relativePath}`,
    );
  }
  let offset = 12;
  let sawPixels = false;
  const budget = createImageMetadataBudget(relativePath);
  while (offset < buffer.length) {
    if (offset + 8 > buffer.length)
      throw new Error(`Invalid WebP chunk: ${relativePath}`);
    const type = buffer.subarray(offset, offset + 4).toString("ascii");
    const length = buffer.readUInt32LE(offset + 4);
    const end = offset + 8 + length;
    const paddedEnd = end + (length % 2);
    if (paddedEnd > buffer.length)
      throw new Error(`Invalid WebP chunk length: ${relativePath}`);
    const payload = buffer.subarray(offset + 8, end);
    if (["VP8 ", "VP8L", "ANMF"].includes(type)) sawPixels = true;
    if (!["VP8 ", "VP8L"].includes(type)) {
      budget.reserveSource(payload.length);
      budget.addDecoded(printableMetadata(payload));
    }
    offset = paddedEnd;
  }
  if (offset !== buffer.length || !sawPixels) {
    throw new Error(`Incomplete WebP image container: ${relativePath}`);
  }
  return budget.finish();
}

function parseIco(buffer, relativePath) {
  if (
    buffer.length < 22 ||
    buffer.readUInt16LE(0) !== 0 ||
    buffer.readUInt16LE(2) !== 1
  ) {
    throw new Error(`Incomplete ICO container: ${relativePath}`);
  }
  const count = buffer.readUInt16LE(4);
  const tableEnd = 6 + count * 16;
  if (count === 0 || tableEnd > buffer.length)
    throw new Error(`Invalid ICO table: ${relativePath}`);
  const ranges = [];
  const budget = createImageMetadataBudget(relativePath);
  for (let index = 0; index < count; index += 1) {
    const entry = 6 + index * 16;
    const length = buffer.readUInt32LE(entry + 8);
    const offset = buffer.readUInt32LE(entry + 12);
    const end = offset + length;
    if (offset < tableEnd || length === 0 || end > buffer.length) {
      throw new Error(`Invalid ICO image entry: ${relativePath}`);
    }
    ranges.push({ offset, end });
    const image = buffer.subarray(offset, end);
    if (
      image
        .subarray(0, 8)
        .equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))
    ) {
      parsePng(image, relativePath, budget);
    } else {
      budget.reserveSource(image.length);
      budget.addDecoded(printableMetadata(image));
    }
  }
  ranges.sort((left, right) => left.offset - right.offset);
  let cursor = tableEnd;
  for (const range of ranges) {
    if (range.offset !== cursor) {
      throw new Error(
        `ICO has overlapping or hidden payload data: ${relativePath}`,
      );
    }
    cursor = range.end;
  }
  if (cursor !== buffer.length)
    throw new Error(`ICO has trailing container data: ${relativePath}`);
  return budget.finish();
}

function parseIcns(buffer, relativePath) {
  if (buffer.length < 8 || buffer.readUInt32BE(4) !== buffer.length) {
    throw new Error(`ICNS length does not match container: ${relativePath}`);
  }
  let offset = 8;
  let sawImage = false;
  const budget = createImageMetadataBudget(relativePath);
  while (offset < buffer.length) {
    if (offset + 8 > buffer.length)
      throw new Error(`Invalid ICNS chunk: ${relativePath}`);
    const length = buffer.readUInt32BE(offset + 4);
    if (length < 8 || offset + length > buffer.length)
      throw new Error(`Invalid ICNS chunk length: ${relativePath}`);
    const payload = buffer.subarray(offset + 8, offset + length);
    if (
      payload
        .subarray(0, 8)
        .equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))
    ) {
      sawImage = true;
      parsePng(payload, relativePath, budget);
    } else {
      budget.reserveSource(payload.length);
      budget.addDecoded(printableMetadata(payload));
    }
    offset += length;
  }
  if (!sawImage) {
    throw new Error(
      `ICNS contains no recognized image payload: ${relativePath}`,
    );
  }
  return budget.finish();
}

export function inspectKnownImage(relativePath, buffer) {
  const extension = path.posix.extname(relativePath).toLowerCase();
  if (!OPAQUE_BINARY_EXTENSIONS.has(extension)) return undefined;
  if (
    buffer
      .subarray(0, 8)
      .equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))
  )
    return parsePng(buffer, relativePath);
  if (buffer[0] === 0xff && buffer[1] === 0xd8 && buffer[2] === 0xff)
    return parseJpeg(buffer, relativePath);
  if (
    buffer.subarray(0, 4).toString("ascii") === "RIFF" &&
    buffer.subarray(8, 12).toString("ascii") === "WEBP"
  )
    return parseWebp(buffer, relativePath);
  if (buffer.subarray(0, 4).equals(Buffer.from([0, 0, 1, 0])))
    return parseIco(buffer, relativePath);
  if (buffer.subarray(0, 4).toString("ascii") === "icns")
    return parseIcns(buffer, relativePath);
  throw new Error(`Unknown image container: ${relativePath}`);
}

export function validateRasterAssetInventory(currentPaths, activeTask) {
  const current = currentPaths
    .filter(
      (relativePath) =>
        !isExcludedPath(relativePath, activeTask) &&
        OPAQUE_BINARY_EXTENSIONS.has(
          path.posix.extname(relativePath).toLowerCase(),
        ),
    )
    .sort((left, right) => left.localeCompare(right, "en"));
  const expected = RASTER_ASSET_CONTRACT.map((asset) => asset.path);
  const findings = [];
  for (const relativePath of current.filter(
    (candidate) => !RASTER_ASSET_DIGESTS.has(candidate),
  )) {
    findings.push(
      finding(
        relativePath,
        0,
        "raster:inventory-drift",
        "unreviewed raster asset",
      ),
    );
  }
  for (const relativePath of expected.filter(
    (candidate) => !current.includes(candidate),
  )) {
    findings.push(
      finding(
        relativePath,
        0,
        "raster:inventory-drift",
        "reviewed raster asset missing",
      ),
    );
  }
  return findings;
}

function textFromBuffer(buffer, relativePath) {
  let source;
  if (buffer[0] === 0xff && buffer[1] === 0xfe) {
    source = new TextDecoder("utf-16le", { fatal: true }).decode(
      buffer.subarray(2),
    );
  } else if (buffer[0] === 0xfe && buffer[1] === 0xff) {
    source = new TextDecoder("utf-16be", { fatal: true }).decode(
      buffer.subarray(2),
    );
  } else {
    const payload =
      buffer[0] === 0xef && buffer[1] === 0xbb && buffer[2] === 0xbf
        ? buffer.subarray(3)
        : buffer;
    if (payload.includes(0)) {
      throw new Error(
        `NUL-containing text requires a supported byte-order mark: ${relativePath}`,
      );
    }
    source = new TextDecoder("utf-8", { fatal: true }).decode(payload);
  }
  if (source.includes("\0")) {
    throw new Error(`Decoded text contains NUL: ${relativePath}`);
  }
  return source;
}

function finding(relativePath, line, rule, excerpt) {
  return {
    path: relativePath,
    line,
    rule,
    excerpt: excerpt.trim().slice(0, 240),
  };
}

export function scanPath(relativePath) {
  normalizeRepositoryPath(relativePath);
  const findings = [];
  for (const rule of PATH_RULES) {
    if (rule.pattern.test(relativePath)) {
      findings.push(finding(relativePath, 0, `path:${rule.id}`, relativePath));
    }
  }
  if (DIRECTORY_PATH_RULE.pattern.test(relativePath)) {
    findings.push(
      finding(relativePath, 0, `path:${DIRECTORY_PATH_RULE.id}`, relativePath),
    );
  }
  return findings;
}

export function validateArchiveEntry(root, relativePath, io = fs, indexMode) {
  normalizeRepositoryPath(relativePath);
  if (!isArchivePath(relativePath)) {
    throw new Error("Archive validation requires an archive path");
  }
  const remainder = relativePath.slice(ARCHIVE_PREFIX.length);
  const parts = remainder.split("/");
  const payload = parts.slice(2);
  const fileName = payload.at(-1) ?? "";
  const validLocation =
    /^\d{4}-\d{2}$/u.test(parts[0] ?? "") &&
    /^\d{2}(?:-\d{2})?-[a-z0-9]+(?:-[a-z0-9]+)*$/u.test(parts[1] ?? "") &&
    (payload.length === 1 ||
      (payload.length === 2 && payload[0] === "research"));
  const extension = path.posix.extname(fileName).toLowerCase();
  const researchJson =
    extension === ".json" && payload.length === 2 && payload[0] === "research";
  const validDocument =
    extension === ".md" ||
    (extension === ".json" && fileName === "task.json") ||
    researchJson ||
    (extension === ".jsonl" &&
      (fileName === "check.jsonl" || fileName === "implement.jsonl"));
  if (!validLocation || !validDocument) {
    throw new Error(
      `Archive payload is not a standard task document: ${relativePath}`,
    );
  }
  const absolute = path.join(root, ...relativePath.split("/"));
  const stat = io.lstatSync(absolute);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(`Archive payload must be a regular file: ${relativePath}`);
  }
  if (indexMode !== "100644") {
    throw new Error(
      `Archive payload has unsafe Git index mode ${indexMode}: ${relativePath}`,
    );
  }
  if ((stat.mode & 0o111) !== 0) {
    throw new Error(`Archive payload must not be executable: ${relativePath}`);
  }
  if (researchJson) {
    JSON.parse(textFromBuffer(io.readFileSync(absolute), relativePath));
  }
  // The canonical archive identity and historical document names are part of
  // the user-approved historical exclusion. Structure, file type, symlink,
  // and executable checks above keep the prefix from becoming an opaque
  // runtime payload area without rewriting or rejecting historical names.
  return [];
}

export function scanText(relativePath, source) {
  normalizeRepositoryPath(relativePath);
  const inspected = relativePath.toLowerCase().endsWith(".svg")
    ? stripOpaqueSvgPayload(source)
    : source;
  const findings = [];
  const skipIds = DEVELOPMENT_HOST_ADMISSION_PATHS.includes(relativePath)
    ? DEVELOPMENT_HOST_CONTENT_RULE_IDS
    : undefined;
  for (const [index, line] of inspected.split(/\r?\n/u).entries()) {
    for (const rule of CONTENT_RULES) {
      if (skipIds?.has(rule.id)) continue;
      if (rule.pattern.test(line)) {
        findings.push(finding(relativePath, index + 1, rule.id, line));
      }
    }
  }
  return findings;
}

function nextNonblank(lines, index) {
  for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
    const value = lines[cursor].trim();
    if (value !== "") return value;
  }
  return "";
}

function matchingRustParenthesis(source, opening) {
  if (source[opening] !== "(") return undefined;
  let depth = 1;
  for (let cursor = opening + 1; cursor < source.length; cursor += 1) {
    if (source[cursor] === "(") depth += 1;
    if (source[cursor] === ")") depth -= 1;
    if (depth === 0) return cursor;
  }
  return undefined;
}

function hasNegatedRustPlatformSelector(compact) {
  for (const match of compact.matchAll(/\bnot\(/gu)) {
    const opening = match.index + match[0].length - 1;
    const closing = matchingRustParenthesis(compact, opening);
    if (closing === undefined) return true;
    const inner = compact.slice(opening + 1, closing);
    if (/(?:target_os|target_family|windows|macos)/u.test(inner)) return true;
  }
  return false;
}

function isImplicitRustCondition(attribute) {
  if (whole(SURFACE_MARKERS.broadRustFamily).test(attribute)) return true;
  const compact = attribute.replace(/\s+/gu, "");
  if (/target_family=/u.test(compact)) return true;
  if (/target_(?:arch|vendor|env)=/u.test(compact)) return true;
  const operatingSystems = Array.from(
    compact.matchAll(/target_os=["']([^"']+)["']/gu),
    (match) => match[1],
  );
  if (
    operatingSystems.some(
      (operatingSystem) =>
        operatingSystem !== "windows" && operatingSystem !== "macos",
    )
  ) {
    return true;
  }
  return hasNegatedRustPlatformSelector(compact);
}

function rustAttributeAt(lines, index) {
  const first = lines[index];
  if (!/#\s*\[\s*cfg(?:_attr)?\s*\(/u.test(first)) return undefined;
  const collected = [];
  let squareBalance = 0;
  for (let cursor = index; cursor < lines.length; cursor += 1) {
    const line = lines[cursor];
    collected.push(line.trim());
    squareBalance += (line.match(/\[/gu) ?? []).length;
    squareBalance -= (line.match(/\]/gu) ?? []).length;
    if (squareBalance === 0) return collected.join(" ");
  }
  return collected.join(" ");
}

export function scanRustImplicitPredicates(entries) {
  const findings = [];
  const seen = new Set();
  const architectureCounts = new Map();
  const macroCounts = new Map();

  const cfgMacroSites = (source) => {
    const sites = [];
    for (const match of source.matchAll(/\bcfg!\s*\(/gu)) {
      const opening = source.indexOf("(", match.index);
      const closing = matchingRustParenthesis(source, opening);
      if (closing === undefined) {
        sites.push({
          index: match.index,
          raw: source.slice(match.index),
          expression: "",
        });
        continue;
      }
      let rawStart = match.index;
      let prefix = match.index - 1;
      while (prefix >= 0 && /\s/u.test(source[prefix])) prefix -= 1;
      const unary = source[prefix] === "!";
      if (unary) rawStart = prefix;
      const raw = source.slice(rawStart, closing + 1);
      sites.push({
        index: rawStart,
        raw,
        expression:
          `${unary ? "!" : ""}cfg!(${source.slice(opening + 1, closing)})`.replace(
            /\s+/gu,
            "",
          ),
      });
    }
    return sites;
  };

  for (const entry of entries) {
    if (!entry.path.endsWith(".rs")) continue;
    const lines = entry.source.split(/\r?\n/u);
    for (const [index, original] of lines.entries()) {
      const attribute = rustAttributeAt(lines, index);
      if (!attribute) continue;
      const compactAttribute = attribute.replace(/\s+/gu, "");
      if (/target_(?:arch|vendor|env)=/u.test(compactAttribute)) {
        const architecture = RUST_ARCH_CFG_CONTRACT.find(
          (candidate) =>
            candidate.file === entry.path &&
            candidate.expression === compactAttribute,
        );
        if (architecture) {
          const key = `${architecture.file}\0${architecture.expression}`;
          architectureCounts.set(key, (architectureCounts.get(key) ?? 0) + 1);
          continue;
        }
      }
      if (!isImplicitRustCondition(attribute)) continue;
      const adjacent = nextNonblank(lines, index);
      const allowance = RUST_ALLOWANCE_CONTRACT.find(
        (candidate) =>
          !seen.has(candidate.id) &&
          candidate.file === entry.path &&
          candidate.condition === attribute &&
          (candidate.nextPrefix
            ? adjacent.startsWith(candidate.next)
            : adjacent === candidate.next),
      );
      if (allowance) {
        seen.add(allowance.id);
      } else {
        findings.push(
          finding(entry.path, index + 1, "rust:implicit-target", attribute),
        );
      }
    }
    for (const site of cfgMacroSites(entry.source)) {
      const expression = site.expression;
      if (
        !/(?:target_(?:os|family|arch|vendor|env)|\bwindows\b|\bmacos\b)/u.test(
          expression,
        )
      ) {
        continue;
      }
      const key = `${entry.path}\0${expression}`;
      macroCounts.set(key, (macroCounts.get(key) ?? 0) + 1);
      const allowance = RUST_CFG_MACRO_CONTRACT.find(
        (contract) =>
          contract.file === entry.path && contract.expression === expression,
      );
      if (allowance) continue;
      const line = entry.source.slice(0, site.index).split(/\r?\n/u).length;
      findings.push(
        finding(entry.path, line, "rust:implicit-target", site.raw),
      );
    }
  }

  for (const contract of RUST_CFG_MACRO_CONTRACT) {
    const count =
      macroCounts.get(`${contract.file}\0${contract.expression}`) ?? 0;
    const normalizedSource = entries
      .find((entry) => entry.path === contract.file)
      ?.source.replace(/\s+/gu, "");
    const anchorsMatch = contract.anchors.every(
      (anchor) =>
        normalizedSource?.split(anchor.replace(/\s+/gu, "")).length - 1 === 1,
    );
    if (count !== contract.count || !anchorsMatch) {
      findings.push(
        finding(
          contract.file,
          0,
          "rust:cfg-macro-drift",
          `${contract.expression}:${count}`,
        ),
      );
    }
  }
  for (const contract of RUST_ARCH_CFG_CONTRACT) {
    const count =
      architectureCounts.get(`${contract.file}\0${contract.expression}`) ?? 0;
    const normalizedSource = entries
      .find((entry) => entry.path === contract.file)
      ?.source.replace(/\s+/gu, "");
    const anchorsMatch = contract.anchors.every(
      (anchor) =>
        normalizedSource?.split(anchor.replace(/\s+/gu, "")).length - 1 === 1,
    );
    if (count !== contract.count || !anchorsMatch) {
      findings.push(
        finding(
          contract.file,
          0,
          "rust:architecture-contract-drift",
          `${contract.expression}:${count}`,
        ),
      );
    }
  }

  const normalizedEntries = new Map(
    entries.map((entry) => [entry.path, entry.source.replace(/\s+/gu, "")]),
  );
  const manualOccurrences = entries.flatMap((entry) =>
    Array.from(
      entry.source.matchAll(new RegExp(MANUAL_TARGET_MARKER, "gu")),
      () => entry.path,
    ),
  );
  for (const file of new Set(manualOccurrences)) {
    if (
      !RUST_MANUAL_TARGET_CONTRACT.some((contract) => contract.file === file)
    ) {
      findings.push(
        finding(
          file,
          0,
          "rust:manual-target",
          "unexpected Cargo target OS read",
        ),
      );
    }
  }
  for (const contract of RUST_MANUAL_TARGET_CONTRACT) {
    const count = manualOccurrences.filter(
      (file) => file === contract.file,
    ).length;
    const normalizedSnippet = contract.snippet.replace(/\s+/gu, "");
    const snippetCount = normalizedEntries.has(contract.file)
      ? normalizedEntries.get(contract.file).split(normalizedSnippet).length - 1
      : 0;
    if (count !== contract.count || snippetCount !== 1) {
      findings.push(
        finding(
          contract.file,
          0,
          "rust:manual-target-contract-drift",
          `${count}:${snippetCount}`,
        ),
      );
    }
  }
  const stdOsPattern = /\bstd::env::consts::OS\b/gu;
  const stdOsOccurrences = entries.flatMap((entry) =>
    Array.from(entry.source.matchAll(stdOsPattern), (match) => ({
      file: entry.path,
      index: match.index,
    })),
  );
  for (const occurrence of stdOsOccurrences) {
    if (occurrence.file !== RUST_STD_OS_CONTRACT.file) {
      const entry = entries.find(
        (candidate) => candidate.path === occurrence.file,
      );
      const line = entry
        ? entry.source.slice(0, occurrence.index).split(/\r?\n/u).length
        : 0;
      findings.push(
        finding(
          occurrence.file,
          line,
          "rust:manual-target",
          "unexpected std OS runtime selector",
        ),
      );
    }
  }
  const stdOsEntry = entries.find(
    (entry) => entry.path === RUST_STD_OS_CONTRACT.file,
  );
  const stdOsCount = stdOsOccurrences.filter(
    (occurrence) => occurrence.file === RUST_STD_OS_CONTRACT.file,
  ).length;
  const stdOsAnchorCount = stdOsEntry
    ? stdOsEntry.source
        .replace(/\s+/gu, "")
        .split(RUST_STD_OS_CONTRACT.anchor.replace(/\s+/gu, "")).length - 1
    : 0;
  if (stdOsCount !== RUST_STD_OS_CONTRACT.count || stdOsAnchorCount !== 1) {
    findings.push(
      finding(
        RUST_STD_OS_CONTRACT.file,
        0,
        "rust:manual-target-contract-drift",
        `std-os:${stdOsCount}:${stdOsAnchorCount}`,
      ),
    );
  }

  for (const allowance of RUST_ALLOWANCE_CONTRACT) {
    if (!seen.has(allowance.id)) {
      findings.push(
        finding(allowance.file, 0, "rust:allowance-drift", allowance.id),
      );
    }
  }
  return findings;
}

function decodeTomlBasicString(source) {
  let output = "";
  for (let index = 0; index < source.length; index += 1) {
    if (source[index] !== "\\") {
      output += source[index];
      continue;
    }
    index += 1;
    const escaped = source[index];
    const simple = {
      b: "\b",
      t: "\t",
      n: "\n",
      f: "\f",
      r: "\r",
      '"': '"',
      "\\": "\\",
    }[escaped];
    if (simple !== undefined) {
      output += simple;
      continue;
    }
    const digits = escaped === "u" ? 4 : escaped === "U" ? 8 : 0;
    const value = source.slice(index + 1, index + 1 + digits);
    if (
      digits === 0 ||
      !new RegExp(`^[0-9a-fA-F]{${digits}}$`, "u").test(value)
    ) {
      throw new Error("Invalid TOML basic-string escape");
    }
    const codePoint = Number.parseInt(value, 16);
    if (codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) {
      throw new Error("Invalid TOML basic-string code point");
    }
    output += String.fromCodePoint(codePoint);
    index += digits;
  }
  return output;
}

function parseTomlDottedKey(source) {
  let cursor = 0;
  const segments = [];
  const skip = () => {
    while (/\s/u.test(source[cursor] ?? "")) cursor += 1;
  };
  while (cursor < source.length) {
    skip();
    let segment;
    if (source[cursor] === "'") {
      const closing = source.indexOf("'", cursor + 1);
      if (closing < 0) throw new Error("Unclosed TOML literal key");
      segment = source.slice(cursor + 1, closing);
      cursor = closing + 1;
    } else if (source[cursor] === '"') {
      let raw = "";
      cursor += 1;
      let closed = false;
      while (cursor < source.length) {
        if (source[cursor] === "\\") {
          const escapeLength = ["u", "U"].includes(source[cursor + 1])
            ? source[cursor + 1] === "u"
              ? 6
              : 10
            : 2;
          raw += source.slice(cursor, cursor + escapeLength);
          cursor += escapeLength;
          continue;
        }
        if (source[cursor] === '"') {
          cursor += 1;
          closed = true;
          break;
        }
        raw += source[cursor];
        cursor += 1;
      }
      if (!closed) throw new Error("Unclosed TOML basic key");
      segment = decodeTomlBasicString(raw);
    } else {
      const match = /^[A-Za-z0-9_-]+/u.exec(source.slice(cursor));
      if (!match) throw new Error("Invalid TOML bare key");
      segment = match[0];
      cursor += match[0].length;
    }
    segments.push(segment);
    skip();
    if (cursor === source.length) break;
    if (source[cursor] !== ".") throw new Error("Invalid TOML dotted key");
    cursor += 1;
  }
  return segments;
}

function parseCargoTargetHeader(line) {
  const trimmed = line.trim();
  if (!trimmed.startsWith("[")) return undefined;
  if (trimmed.startsWith("[[") || !trimmed.endsWith("]")) {
    throw new Error("Invalid Cargo table header");
  }
  const segments = parseTomlDottedKey(trimmed.slice(1, -1));
  if (segments[0] !== "target") return undefined;
  if (
    segments.length !== 3 ||
    !/^(?:build-|dev-)?dependencies$/u.test(segments[2])
  ) {
    throw new Error("Invalid Cargo target table shape");
  }
  return segments[1];
}

function cargoTargetAssignment(line) {
  const trimmed = line.trim();
  if (trimmed === "" || trimmed.startsWith("#") || trimmed.startsWith("[")) {
    return false;
  }
  let quote;
  let escaped = false;
  let equals = -1;
  for (let index = 0; index < trimmed.length; index += 1) {
    const value = trimmed[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (quote === '"' && value === "\\") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (value === quote) quote = undefined;
      continue;
    }
    if (value === '"' || value === "'") {
      quote = value;
      continue;
    }
    if (value === "=") {
      equals = index;
      break;
    }
  }
  if (equals < 0) return false;
  try {
    return parseTomlDottedKey(trimmed.slice(0, equals).trim())[0] === "target";
  } catch {
    return /target/iu.test(trimmed.slice(0, equals));
  }
}

function parseCargoCfgExpression(selector) {
  let cursor = 0;
  const skip = () => {
    while (/\s/u.test(selector[cursor] ?? "")) cursor += 1;
  };
  const parseIdentifier = () => {
    skip();
    const match = /^[A-Za-z_][A-Za-z0-9_]*/u.exec(selector.slice(cursor));
    if (!match) throw new Error("Expected cfg identifier");
    cursor += match[0].length;
    return match[0];
  };
  const parseString = () => {
    skip();
    const quote = selector[cursor];
    if (quote !== '"' && quote !== "'") throw new Error("Expected cfg string");
    cursor += 1;
    let value = "";
    while (cursor < selector.length && selector[cursor] !== quote) {
      if (quote === '"' && selector[cursor] === "\\") {
        const start = cursor;
        cursor += 2;
        value += decodeTomlBasicString(selector.slice(start, cursor));
      } else {
        value += selector[cursor];
        cursor += 1;
      }
    }
    if (selector[cursor] !== quote) throw new Error("Unclosed cfg string");
    cursor += 1;
    return value;
  };
  const parseNode = () => {
    const name = parseIdentifier();
    skip();
    if (selector[cursor] === "=") {
      cursor += 1;
      return { kind: "atom", name, value: parseString() };
    }
    if (selector[cursor] !== "(") return { kind: "atom", name };
    cursor += 1;
    const children = [];
    while (true) {
      skip();
      if (selector[cursor] === ")") {
        cursor += 1;
        break;
      }
      children.push(parseNode());
      skip();
      if (selector[cursor] === ",") {
        cursor += 1;
        continue;
      }
      if (selector[cursor] !== ")")
        throw new Error("Invalid cfg argument list");
    }
    if (!["cfg", "all", "any", "not"].includes(name) || children.length === 0) {
      throw new Error("Unsupported cfg function");
    }
    if ((name === "cfg" || name === "not") && children.length !== 1) {
      throw new Error("Invalid unary cfg function");
    }
    return { kind: name, children };
  };
  const node = parseNode();
  skip();
  if (cursor !== selector.length || node.kind !== "cfg") {
    throw new Error("Invalid Cargo cfg selector");
  }
  return node.children[0];
}

function cargoCfgTruthOnUnsupportedHost(node) {
  if (node.kind === "atom") {
    if (node.value === undefined && ["windows", "macos"].includes(node.name)) {
      return { canTrue: false, canFalse: true };
    }
    if (
      node.name === "target_os" &&
      (node.value === "windows" || node.value === "macos")
    ) {
      return { canTrue: false, canFalse: true };
    }
    return { canTrue: true, canFalse: true };
  }
  const values = node.children.map(cargoCfgTruthOnUnsupportedHost);
  if (node.kind === "not") {
    return { canTrue: values[0].canFalse, canFalse: values[0].canTrue };
  }
  if (node.kind === "all") {
    return {
      canTrue: values.every((value) => value.canTrue),
      canFalse: values.some((value) => value.canFalse),
    };
  }
  return {
    canTrue: values.some((value) => value.canTrue),
    canFalse: values.every((value) => value.canFalse),
  };
}

export function scanCargoImplicitPredicates(entries) {
  const findings = [];
  for (const entry of entries) {
    if (!entry.path.endsWith("Cargo.toml")) continue;
    for (const [index, line] of entry.source.split(/\r?\n/u).entries()) {
      const trimmed = line.trim();
      const targetAssignment = cargoTargetAssignment(line);
      if (!trimmed.startsWith("[") && !targetAssignment) continue;
      if (targetAssignment) {
        findings.push(
          finding(entry.path, index + 1, "cargo:implicit-target", line),
        );
        continue;
      }
      let implicit = true;
      try {
        const selector = parseCargoTargetHeader(line);
        if (selector === undefined) continue;
        if (selector?.startsWith("cfg")) {
          implicit = cargoCfgTruthOnUnsupportedHost(
            parseCargoCfgExpression(selector),
          ).canTrue;
        }
      } catch {
        if (!/target/iu.test(line) && !/\\u0*067/iu.test(line)) continue;
      }
      if (implicit) {
        findings.push(
          finding(entry.path, index + 1, "cargo:implicit-target", line),
        );
      }
    }
  }
  return findings;
}

export function scanMacosPosixContract(entries) {
  const findings = [];
  for (const contract of MACOS_POSIX_CONTRACT) {
    const entry = entries.find(
      ({ path: entryPath }) => entryPath === contract.file,
    );
    const count = entry ? entry.source.split(contract.snippet).length - 1 : 0;
    if (count !== 1) {
      findings.push(
        finding(contract.file, 0, "macos-posix:contract-drift", contract.id),
      );
    }
  }
  return findings;
}

export function scanDirectoryConventionContract(entries) {
  const findings = [];
  const occurrences = [];
  for (const entry of entries) {
    const inspected = entry.path.toLowerCase().endsWith(".svg")
      ? stripOpaqueSvgPayload(entry.source)
      : entry.source;
    for (const line of inspected.split(/\r?\n/u)) {
      if (DIRECTORY_MARKER_PATTERN.test(line)) {
        occurrences.push({ file: entry.path, snippet: line.trim() });
      }
    }
  }
  for (const occurrence of occurrences) {
    if (
      !DIRECTORY_OCCURRENCE_CONTRACT.some(
        (contract) =>
          contract.file === occurrence.file &&
          contract.snippet === occurrence.snippet,
      )
    ) {
      findings.push(
        finding(
          occurrence.file,
          0,
          "macos-posix:unexpected-variable",
          occurrence.snippet,
        ),
      );
    }
  }
  for (const contract of DIRECTORY_OCCURRENCE_CONTRACT) {
    const count = occurrences.filter(
      (occurrence) =>
        occurrence.file === contract.file &&
        occurrence.snippet === contract.snippet,
    ).length;
    const entry = entries.find((candidate) => candidate.path === contract.file);
    const anchorCount = contract.anchor
      ? (entry?.source
          .replace(/\s+/gu, "")
          .split(contract.anchor.replace(/\s+/gu, "")).length ?? 1) - 1
      : 1;
    if (count !== contract.count || anchorCount !== 1) {
      findings.push(
        finding(
          contract.file,
          0,
          "macos-posix:contract-drift",
          `${contract.snippet}:${count}`,
        ),
      );
    }
  }
  const identifierPattern = new RegExp(`\\b${DATA_HOME_IDENTIFIER}\\b`, "u");
  const referenceOccurrences = entries.flatMap((entry) =>
    entry.source
      .split(/\r?\n/u)
      .filter((line) => identifierPattern.test(line))
      .map((line) => ({ file: entry.path, snippet: line.trim() })),
  );
  for (const occurrence of referenceOccurrences) {
    if (
      !DATA_HOME_REFERENCE_CONTRACT.some(
        (contract) =>
          contract.file === occurrence.file &&
          contract.snippet === occurrence.snippet,
      )
    ) {
      findings.push(
        finding(
          occurrence.file,
          0,
          "macos-posix:unexpected-variable",
          occurrence.snippet,
        ),
      );
    }
  }
  for (const contract of DATA_HOME_REFERENCE_CONTRACT) {
    const count = referenceOccurrences.filter(
      (occurrence) =>
        occurrence.file === contract.file &&
        occurrence.snippet === contract.snippet,
    ).length;
    const entry = entries.find((candidate) => candidate.path === contract.file);
    const anchorCount = contract.anchor
      ? (entry?.source
          .replace(/\s+/gu, "")
          .split(contract.anchor.replace(/\s+/gu, "")).length ?? 1) - 1
      : 1;
    if (count !== contract.count || anchorCount !== 1) {
      findings.push(
        finding(
          contract.file,
          0,
          "macos-posix:contract-drift",
          `data-home-identifier-references:${count}`,
        ),
      );
    }
  }
  return findings;
}

function javascriptSource(relativePath) {
  return /\.(?:js|jsx|mjs|cjs|ts|tsx|mts|cts)$/iu.test(relativePath);
}

function maskJavaScriptData(source) {
  // Every scanner cursor below uses JavaScript string offsets, which are
  // UTF-16 code-unit indexes. Keep the mask on the same index model so an
  // astral character in inert data cannot shift executable code out of view.
  const output = source.split("");
  const blank = (start, end) => {
    for (let index = start; index < end; index += 1) {
      if (output[index] !== "\n" && output[index] !== "\r") output[index] = " ";
    }
  };
  const significantBefore = (offset) => {
    let cursor = offset - 1;
    while (cursor >= 0 && /\s/u.test(source[cursor])) cursor -= 1;
    return cursor;
  };
  const regexCanStart = (offset) => {
    const previous = significantBefore(offset);
    if (previous < 0) return true;
    if (/[({[=,:;!?&|+\-*%^~<>]/u.test(source[previous])) return true;
    const prefix = source.slice(0, previous + 1);
    const keyword =
      /\b(?:return|throw|case|delete|typeof|void|new|yield|await|else|do)\s*$/u.exec(
        prefix,
      );
    return Boolean(keyword);
  };

  const scanQuoted = (start, quote) => {
    let cursor = start + 1;
    while (cursor < source.length) {
      if (source[cursor] === "\\") {
        cursor += 2;
        continue;
      }
      if (source[cursor] === quote) return cursor + 1;
      cursor += 1;
    }
    return source.length;
  };

  const scanRegex = (start) => {
    let cursor = start + 1;
    let inClass = false;
    while (cursor < source.length) {
      if (source[cursor] === "\\") {
        cursor += 2;
        continue;
      }
      if (source[cursor] === "[") inClass = true;
      if (source[cursor] === "]") inClass = false;
      if (source[cursor] === "/" && !inClass) {
        cursor += 1;
        while (/[A-Za-z]/u.test(source[cursor] ?? "")) cursor += 1;
        return cursor;
      }
      if (source[cursor] === "\n" || source[cursor] === "\r") return start + 1;
      cursor += 1;
    }
    return start + 1;
  };

  const scanCode = (start, stopAtTemplateBrace = false) => {
    let braceDepth = 0;
    let index = start;
    while (index < source.length) {
      if (stopAtTemplateBrace && source[index] === "}" && braceDepth === 0) {
        return index;
      }
      if (source[index] === "/" && source[index + 1] === "/") {
        const end = source.indexOf("\n", index + 2);
        const boundary = end < 0 ? source.length : end;
        blank(index, boundary);
        index = boundary;
        continue;
      }
      if (source[index] === "/" && source[index + 1] === "*") {
        const closing = source.indexOf("*/", index + 2);
        const boundary = closing < 0 ? source.length : closing + 2;
        blank(index, boundary);
        index = boundary;
        continue;
      }
      if (source[index] === "/" && regexCanStart(index)) {
        const boundary = scanRegex(index);
        if (boundary > index + 1) {
          blank(index, boundary);
          index = boundary;
          continue;
        }
      }
      if (source[index] === '"' || source[index] === "'") {
        const quote = source[index];
        const boundary = scanQuoted(index, quote);
        const literal = source.slice(index + 1, boundary - 1);
        if (!["win32", "darwin", "unknown", "platform"].includes(literal)) {
          blank(index, boundary);
        }
        index = boundary;
        continue;
      }
      if (source[index] === "`") {
        blank(index, index + 1);
        let cursor = index + 1;
        let quasiStart = cursor;
        while (cursor < source.length) {
          if (source[cursor] === "\\") {
            cursor += 2;
            continue;
          }
          if (source[cursor] === "`") {
            blank(quasiStart, cursor + 1);
            cursor += 1;
            break;
          }
          if (source[cursor] === "$" && source[cursor + 1] === "{") {
            blank(quasiStart, cursor + 2);
            output[cursor + 1] = "(";
            const closing = scanCode(cursor + 2, true);
            if (closing >= source.length) {
              cursor = source.length;
              break;
            }
            output[closing] = ")";
            cursor = closing + 1;
            quasiStart = cursor;
            continue;
          }
          cursor += 1;
        }
        index = cursor;
        continue;
      }
      if (stopAtTemplateBrace && source[index] === "{") braceDepth += 1;
      if (stopAtTemplateBrace && source[index] === "}") braceDepth -= 1;
      index += 1;
    }
    return source.length;
  };

  scanCode(0);
  return output.join("");
}

function skipJavaScriptWhitespace(source, start) {
  let cursor = start;
  while (/\s/u.test(source[cursor] ?? "")) cursor += 1;
  return cursor;
}

function matchingJavaScriptDelimiter(source, opening, open, close) {
  if (source[opening] !== open) return undefined;
  let depth = 1;
  for (let cursor = opening + 1; cursor < source.length; cursor += 1) {
    if (source[cursor] === open) depth += 1;
    if (source[cursor] === close) depth -= 1;
    if (depth === 0) return cursor;
  }
  return undefined;
}

function javascriptStatementEnd(source, start) {
  let cursor = start;
  cursor = skipJavaScriptWhitespace(source, cursor);
  if (source[cursor] !== "{") {
    let parentheses = 0;
    let brackets = 0;
    let braces = 0;
    for (; cursor < source.length; cursor += 1) {
      if (source[cursor] === "(") parentheses += 1;
      if (source[cursor] === ")") parentheses -= 1;
      if (source[cursor] === "[") brackets += 1;
      if (source[cursor] === "]") brackets -= 1;
      if (source[cursor] === "{") braces += 1;
      if (source[cursor] === "}") {
        if (braces === 0 && parentheses === 0 && brackets === 0) return cursor;
        braces -= 1;
      }
      if (
        source[cursor] === ";" &&
        parentheses === 0 &&
        brackets === 0 &&
        braces === 0
      ) {
        return cursor + 1;
      }
    }
    return cursor;
  }
  const closing = matchingJavaScriptDelimiter(source, cursor, "{", "}");
  return closing === undefined ? undefined : closing + 1;
}

function unwrapJavaScriptBlock(source) {
  let value = source.trim();
  if (value.startsWith("default")) {
    value = value.replace(/^default\s*:/u, "").trim();
  }
  if (value.startsWith("{")) {
    const closing = matchingJavaScriptDelimiter(value, 0, "{", "}");
    if (closing === value.length - 1) value = value.slice(1, -1).trim();
  }
  return value;
}

function isFailClosedJavaScript(value, expressionOnly = false) {
  const normalized = unwrapJavaScriptBlock(value);
  if (/^(?:null|undefined|false|["']unknown["'])\s*;?$/u.test(normalized)) {
    return true;
  }
  if (expressionOnly) return false;
  if (
    /^return\s+(?:null|undefined|false|["']unknown["'])\s*;?$/u.test(normalized)
  ) {
    return true;
  }
  if (!/^throw\b/u.test(normalized)) return false;
  const end = javascriptStatementEnd(normalized, 0);
  return end !== undefined && normalized.slice(end).trim() === "";
}

const PROCESS_PLATFORM_SELECTOR = combine(
  "(?:process\\s*(?:\\?\\.|\\.)\\s*platform|",
  "process\\s*(?:\\?\\.)?\\s*\\[\\s*[\"']platform[\"']\\s*\\]|",
  "(?<![A-Za-z0-9_$])platform)",
);
const WINDOWS_HELPER_SELECTOR =
  "(?:[A-Za-z_$][\\w$]*\\s*(?:\\?\\.|\\.)\\s*)*isWindows\\s*(?:\\?\\.)?\\s*\\([^)]*\\)";
const MACOS_HELPER_SELECTOR =
  "(?:[A-Za-z_$][\\w$]*\\s*(?:\\?\\.|\\.)\\s*)*isMac(?:OS)?\\s*(?:\\?\\.)?\\s*\\([^)]*\\)";

const JAVASCRIPT_PLATFORM_EXPRESSION_CONTRACT = Object.freeze([
  Object.freeze({
    file: "src/App.tsx",
    expression: "const DEFAULT_DRAG_BAR_HEIGHT = isMac() ? 28 : 0",
  }),
  Object.freeze({
    file: "src/components/common/FullScreenPanel.tsx",
    expression: "const DRAG_BAR_HEIGHT = isMac() ? 28 : 0",
  }),
]);

function isApprovedJavaScriptPlatformExpression(entry, expression) {
  const normalizedExpression = expression.replace(/\s+/gu, " ").trim();
  return JAVASCRIPT_PLATFORM_EXPRESSION_CONTRACT.some((contract) => {
    if (
      contract.file !== entry.path ||
      normalizedExpression !== contract.expression
    ) {
      return false;
    }
    const normalizedSource = entry.source.replace(/\s+/gu, " ");
    return normalizedSource.split(contract.expression).length - 1 === 1;
  });
}

function classifyJavaScriptPlatformPredicate(condition) {
  const classify = (platform, literal, helper) => {
    const positive = new RegExp(
      `(?:${PROCESS_PLATFORM_SELECTOR}\\s*={2,3}\\s*["']${literal}["']|["']${literal}["']\\s*={2,3}\\s*${PROCESS_PLATFORM_SELECTOR}|${helper})`,
      "u",
    );
    const negative = new RegExp(
      `(?:${PROCESS_PLATFORM_SELECTOR}\\s*!={1,2}\\s*["']${literal}["']|["']${literal}["']\\s*!={1,2}\\s*${PROCESS_PLATFORM_SELECTOR}|!\\s*(?:${helper})|(?:${helper})\\s*={2,3}\\s*false|false\\s*={2,3}\\s*(?:${helper}))`,
      "u",
    );
    if (negative.test(condition)) return { platform, negated: true };
    if (positive.test(condition)) return { platform, negated: false };
    return undefined;
  };
  return (
    classify("windows", "win32", WINDOWS_HELPER_SELECTOR) ??
    classify("macos", "darwin", MACOS_HELPER_SELECTOR)
  );
}

function parseJavaScriptIf(source, offset) {
  const afterKeyword = skipJavaScriptWhitespace(source, offset + 2);
  if (source[afterKeyword] !== "(") return undefined;
  const conditionEnd = matchingJavaScriptDelimiter(
    source,
    afterKeyword,
    "(",
    ")",
  );
  if (conditionEnd === undefined) return undefined;
  const bodyStart = skipJavaScriptWhitespace(source, conditionEnd + 1);
  const bodyEnd = javascriptStatementEnd(source, bodyStart);
  if (bodyEnd === undefined) return undefined;
  return {
    condition: source.slice(afterKeyword + 1, conditionEnd),
    bodyStart,
    bodyEnd,
  };
}

function nextJavaScriptFallback(source, start) {
  let cursor = skipJavaScriptWhitespace(source, start);
  if (source.slice(cursor, cursor + 4) === "else") {
    cursor = skipJavaScriptWhitespace(source, cursor + 4);
  }
  if (/^if\b/u.test(source.slice(cursor))) {
    const branch = parseJavaScriptIf(source, cursor);
    return branch ? { kind: "if", offset: cursor, branch } : undefined;
  }
  if (
    !/^(?:\{|throw\b|return\b|const\b|let\b|var\b|[A-Za-z_$][\w$]*(?:\s*\.|\s*\())/u.test(
      source.slice(cursor),
    )
  ) {
    return undefined;
  }
  const end = javascriptStatementEnd(source, cursor);
  return end === undefined
    ? undefined
    : { kind: "fallback", offset: cursor, source: source.slice(cursor, end) };
}

function ternaryColon(source, question, boundary = source.length) {
  let nested = 0;
  let parentheses = 0;
  let brackets = 0;
  let braces = 0;
  for (let cursor = question + 1; cursor < boundary; cursor += 1) {
    const value = source[cursor];
    if (value === "(") parentheses += 1;
    if (value === ")") {
      if (parentheses === 0) return undefined;
      parentheses -= 1;
    }
    if (value === "[") brackets += 1;
    if (value === "]") {
      if (brackets === 0) return undefined;
      brackets -= 1;
    }
    if (value === "{") braces += 1;
    if (value === "}") {
      if (braces === 0) return undefined;
      braces -= 1;
    }
    if (parentheses !== 0 || brackets !== 0 || braces !== 0) continue;
    if (
      value === "?" &&
      source[cursor + 1] !== "." &&
      source[cursor + 1] !== "?"
    ) {
      nested += 1;
    } else if (value === ":") {
      if (nested === 0) return cursor;
      nested -= 1;
    }
  }
  return undefined;
}

function ternaryExpressionEnd(source, start) {
  let parentheses = 0;
  let brackets = 0;
  let braces = 0;
  for (let cursor = start; cursor < source.length; cursor += 1) {
    const value = source[cursor];
    if (value === "(") parentheses += 1;
    if (value === ")") {
      if (parentheses === 0) return cursor;
      parentheses -= 1;
    }
    if (value === "[") brackets += 1;
    if (value === "]") {
      if (brackets === 0) return cursor;
      brackets -= 1;
    }
    if (value === "{") braces += 1;
    if (value === "}") {
      if (braces === 0) return cursor;
      braces -= 1;
    }
    if (
      parentheses === 0 &&
      brackets === 0 &&
      braces === 0 &&
      (value === ";" || value === ",")
    ) {
      return cursor;
    }
  }
  return source.length;
}

function ultimateTernaryFallback(source) {
  const value = source.trim();
  let question = -1;
  for (let index = 0; index < value.length; index += 1) {
    if (
      value[index] === "?" &&
      value[index + 1] !== "." &&
      value[index + 1] !== "?"
    ) {
      question = index;
      break;
    }
  }
  if (question < 0) return value;
  const colon = ternaryColon(value, question);
  if (colon === undefined) return value;
  return ultimateTernaryFallback(value.slice(colon + 1));
}

export function scanJavaScriptImplicitPredicates(entries) {
  const findings = [];
  for (const entry of entries) {
    if (!javascriptSource(entry.path)) continue;
    const source = maskJavaScriptData(entry.source);
    for (const match of source.matchAll(/\bif\b/gu)) {
      const first = parseJavaScriptIf(source, match.index);
      if (!first) continue;
      const firstSelector = classifyJavaScriptPlatformPredicate(
        first.condition,
      );
      if (!firstSelector) continue;
      if (firstSelector.negated) {
        const line = source.slice(0, match.index).split(/\r?\n/u).length;
        findings.push(
          finding(
            entry.path,
            line,
            "js:implicit-target",
            source.slice(match.index, first.bodyEnd),
          ),
        );
        continue;
      }
      let next = nextJavaScriptFallback(source, first.bodyEnd);
      if (next?.kind === "if") {
        const nextSelector = classifyJavaScriptPlatformPredicate(
          next.branch.condition,
        );
        if (
          nextSelector &&
          !nextSelector.negated &&
          nextSelector.platform !== firstSelector.platform
        ) {
          next = nextJavaScriptFallback(source, next.branch.bodyEnd);
        }
      }
      if (next?.kind === "fallback" && !isFailClosedJavaScript(next.source)) {
        const line = source.slice(0, match.index).split(/\r?\n/u).length;
        findings.push(
          finding(
            entry.path,
            line,
            "js:implicit-target",
            source.slice(match.index, first.bodyEnd),
          ),
        );
      }
    }

    for (let question = 0; question < source.length; question += 1) {
      if (
        source[question] !== "?" ||
        source[question + 1] === "." ||
        source[question + 1] === "?"
      ) {
        continue;
      }
      const prefixStart = Math.max(
        source.lastIndexOf(";", question - 1),
        source.lastIndexOf("{", question - 1),
        source.lastIndexOf("}", question - 1),
        question - 600,
      );
      const condition = source.slice(prefixStart + 1, question);
      const selector = classifyJavaScriptPlatformPredicate(condition);
      if (!selector) continue;
      const colon = ternaryColon(source, question);
      if (colon === undefined) continue;
      const end = ternaryExpressionEnd(source, colon + 1);
      const fallback = ultimateTernaryFallback(source.slice(colon + 1, end));
      if (!selector.negated && isFailClosedJavaScript(fallback, true)) continue;
      if (
        isApprovedJavaScriptPlatformExpression(
          entry,
          source.slice(prefixStart + 1, end),
        )
      ) {
        continue;
      }
      const line = source.slice(0, question).split(/\r?\n/u).length;
      findings.push(
        finding(
          entry.path,
          line,
          "js:implicit-target",
          source.slice(prefixStart + 1, end),
        ),
      );
    }

    for (const match of source.matchAll(/\bswitch\b/gu)) {
      const conditionStart = skipJavaScriptWhitespace(
        source,
        match.index + match[0].length,
      );
      if (source[conditionStart] !== "(") continue;
      const conditionEnd = matchingJavaScriptDelimiter(
        source,
        conditionStart,
        "(",
        ")",
      );
      if (conditionEnd === undefined) continue;
      const condition = source.slice(conditionStart + 1, conditionEnd);
      if (
        !new RegExp(`^\s*${PROCESS_PLATFORM_SELECTOR}\s*$`, "u").test(condition)
      ) {
        continue;
      }
      const opening = skipJavaScriptWhitespace(source, conditionEnd + 1);
      if (source[opening] !== "{") continue;
      const closing = matchingJavaScriptDelimiter(source, opening, "{", "}");
      if (closing === undefined) {
        findings.push(
          finding(entry.path, 0, "js:implicit-target", "unclosed switch"),
        );
        continue;
      }
      const body = source.slice(opening + 1, closing);
      if (!/\bcase\s+["'](?:win32|darwin)["']\s*:/u.test(body)) continue;
      let depth = 0;
      let defaultOffset = -1;
      for (let cursor = 0; cursor < body.length; cursor += 1) {
        if (body[cursor] === "{") depth += 1;
        if (body[cursor] === "}") depth -= 1;
        if (depth === 0 && /^default\s*:/u.test(body.slice(cursor))) {
          defaultOffset = cursor;
          break;
        }
      }
      if (defaultOffset < 0) continue;
      const colon = body.indexOf(":", defaultOffset);
      const defaultStart = skipJavaScriptWhitespace(body, colon + 1);
      const defaultEnd = javascriptStatementEnd(body, defaultStart);
      const defaultBody =
        defaultEnd === undefined
          ? body.slice(defaultStart)
          : body.slice(defaultStart, defaultEnd);
      if (!isFailClosedJavaScript(defaultBody)) {
        const line = source.slice(0, match.index).split(/\r?\n/u).length;
        findings.push(
          finding(entry.path, line, "js:implicit-target", "switch default"),
        );
      }
    }
  }
  return findings;
}

export function listCurrentFiles(root = ROOT, runner = spawnSync) {
  const result = runner(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    {
      cwd: root,
      encoding: null,
      windowsHide: true,
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `Unable to enumerate repository files (status ${String(result.status)})`,
    );
  }
  if (!Buffer.isBuffer(result.stdout)) {
    throw new Error("Repository file enumeration returned an invalid payload");
  }
  const decoded = new TextDecoder("utf-8", { fatal: true }).decode(
    result.stdout,
  );
  const files = decoded
    .split("\0")
    .filter(Boolean)
    .map(normalizeRepositoryPath);
  if (new Set(files).size !== files.length) {
    throw new Error("Repository file enumeration returned duplicate paths");
  }
  return files.sort((left, right) => left.localeCompare(right, "en"));
}

function listIndexModes(root, runner, pathspec) {
  const arguments_ = ["ls-files", "--stage", "-z"];
  if (pathspec !== undefined) arguments_.push("--", pathspec);
  const result = runner("git", arguments_, {
    cwd: root,
    encoding: null,
    windowsHide: true,
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0 || !Buffer.isBuffer(result.stdout)) {
    throw new Error("Unable to enumerate Git index modes");
  }
  const decoded = new TextDecoder("utf-8", { fatal: true }).decode(
    result.stdout,
  );
  const modes = new Map();
  for (const record of decoded.split("\0").filter(Boolean)) {
    const match = /^(\d{6}) [0-9a-f]+ 0\t(.+)$/u.exec(record);
    if (!match) throw new Error("Invalid Git index record");
    const relativePath = normalizeRepositoryPath(match[2]);
    if (modes.has(relativePath)) {
      throw new Error(`Duplicate Git index record: ${relativePath}`);
    }
    modes.set(relativePath, match[1]);
  }
  return modes;
}

export function listCurrentIndexModes(root = ROOT, runner = spawnSync) {
  return listIndexModes(root, runner);
}

export function listArchiveIndexModes(root = ROOT, runner = spawnSync) {
  return listIndexModes(root, runner, ARCHIVE_PREFIX);
}

export function readCurrentEntry(root, relativePath, io = fs, indexMode) {
  const absolute = path.join(
    root,
    ...normalizeRepositoryPath(relativePath).split("/"),
  );
  let stat;
  try {
    stat = io.lstatSync(absolute);
  } catch (error) {
    if (error && typeof error === "object" && error.code === "ENOENT") {
      return undefined;
    }
    throw error;
  }

  const extension = path.posix.extname(relativePath).toLowerCase();
  const raster = OPAQUE_BINARY_EXTENSIONS.has(extension);
  if (
    raster &&
    (!stat.isFile() || stat.isSymbolicLink() || indexMode !== "100644")
  ) {
    throw new Error(
      `Raster asset must be a regular 100644 Git file: ${relativePath}`,
    );
  }
  if (stat.isSymbolicLink()) {
    return { path: relativePath, source: io.readlinkSync(absolute, "utf8") };
  }
  if (!stat.isFile()) {
    throw new Error(`Tracked path is not a regular file: ${relativePath}`);
  }
  const buffer = io.readFileSync(absolute);
  if (raster) {
    const expectedDigest = RASTER_ASSET_DIGESTS.get(relativePath);
    const actualDigest = createHash("sha256").update(buffer).digest("hex");
    if (expectedDigest === undefined || actualDigest !== expectedDigest) {
      throw new Error(`Raster asset identity is not reviewed: ${relativePath}`);
    }
  }
  const imageMetadata = inspectKnownImage(relativePath, buffer);
  const source =
    imageMetadata === undefined
      ? textFromBuffer(buffer, relativePath)
      : imageMetadata || undefined;
  return { path: relativePath, source };
}

export function inspectRepository({
  root = ROOT,
  activeTask,
  runner = spawnSync,
  io = fs,
  sessionResolver = resolveAuthoritativeActiveTask,
} = {}) {
  const excludedTask = activeTask
    ? validateActiveTaskExclusion(activeTask, {
        root,
        io,
        runner,
        sessionResolver,
      })
    : undefined;
  const currentPaths = listCurrentFiles(root, runner);
  const currentIndexModes = listCurrentIndexModes(root, runner);
  for (const [manifestPath, label] of [
    [RASTER_MANIFEST_RELATIVE_PATH, "raster asset"],
    [STRUCTURE_MANIFEST_RELATIVE_PATH, "structure asset"],
  ]) {
    if (currentIndexModes.get(manifestPath) !== "100644") {
      throw new Error(
        `Supported-platform ${label} manifest must have Git index mode 100644`,
      );
    }
  }
  validateStructureAssetInventory(currentPaths, currentIndexModes, {
    root,
    io,
    activeTask: excludedTask,
  });
  const findings = [];
  findings.push(...validateRasterAssetInventory(currentPaths, excludedTask));
  const rustEntries = [];
  const javascriptEntries = [];
  const textEntries = [];
  const cargoEntries = [];
  let inspectedFiles = 0;

  for (const relativePath of currentPaths) {
    if (isArchivePath(relativePath)) {
      inspectedFiles += 1;
      findings.push(
        ...validateArchiveEntry(
          root,
          relativePath,
          io,
          currentIndexModes.get(relativePath),
        ),
      );
      continue;
    }
    if (isExcludedPath(relativePath, excludedTask)) continue;
    const entry = readCurrentEntry(
      root,
      relativePath,
      io,
      currentIndexModes.get(relativePath),
    );
    if (!entry) continue;
    inspectedFiles += 1;
    findings.push(...scanPath(relativePath));
    if (entry.source === undefined || isTextExcludedPath(relativePath)) {
      continue;
    }
    findings.push(...scanText(relativePath, entry.source));
    textEntries.push(entry);
    if (relativePath.endsWith(".rs")) rustEntries.push(entry);
    if (relativePath.endsWith("Cargo.toml")) cargoEntries.push(entry);
    if (javascriptSource(relativePath)) javascriptEntries.push(entry);
  }
  findings.push(...scanRustImplicitPredicates(rustEntries));
  findings.push(...scanCargoImplicitPredicates(cargoEntries));
  findings.push(...scanMacosPosixContract(rustEntries));
  findings.push(...scanDirectoryConventionContract(textEntries));
  findings.push(...scanJavaScriptImplicitPredicates(javascriptEntries));
  findings.sort(
    (left, right) =>
      left.path.localeCompare(right.path, "en") ||
      left.line - right.line ||
      left.rule.localeCompare(right.rule, "en"),
  );
  return { findings, inspectedFiles };
}

function main() {
  const activeTask = parseArguments(process.argv.slice(2));
  const report = inspectRepository({ activeTask });
  if (report.findings.length > 0) {
    for (const item of report.findings) {
      const location = item.line > 0 ? `${item.path}:${item.line}` : item.path;
      console.error(`${location} [${item.rule}] ${item.excerpt}`);
    }
    throw new Error(
      `Supported platform surface check found ${report.findings.length} issue(s)`,
    );
  }
  console.log(
    `Supported platform surface check passed (${report.inspectedFiles} current files).`,
  );
}

if (isMain(import.meta.url)) {
  try {
    main();
  } catch (error) {
    fail(error);
  }
}
