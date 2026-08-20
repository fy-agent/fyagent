import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const read = (relativePath: string) =>
  fs.readFileSync(path.join(ROOT, relativePath), "utf8").replace(/\r\n/g, "\n");

const startup = read("src-tauri/src/windows_runtime/native.rs");
const startupDomain = read("src-tauri/src/windows_runtime/mod.rs");
const shellRegistry = read("src-tauri/src/windows_runtime/registry.rs");
const cargoManifest = read("src-tauri/Cargo.toml");
const autoLaunch = read("src-tauri/src/auto_launch.rs");
const envChecker = read("src-tauri/src/services/env_checker.rs");
const envManager = read("src-tauri/src/services/env_manager.rs");
const deployment = read(
  "src-tauri/src/codex_desktop/platform/windows/deployment.rs",
);
const adapter = read("src-tauri/src/codex_desktop/platform/windows/mod.rs");
const helper = read("src-tauri/src/codex_desktop/platform/windows/helper.rs");
const packageBridge = read(
  "src-tauri/src/codex_desktop/platform/windows/package_bridge.rs",
).split("#[cfg(test)]", 1)[0];
const runtime = read("src-tauri/src/codex_desktop/platform/windows/runtime.rs");
const userHelperRuntime = read("src-tauri/user-helper/src/windows.rs").split(
  "#[cfg(test)]",
  1,
)[0];
const userHelperLayout = read("src-tauri/user-helper/src/layout.rs").split(
  "#[cfg(test)]",
  1,
)[0];
const tempRoot = read("src-tauri/src/codex_desktop/temp.rs");
const desktopRuntime = read("src-tauri/src/codex_desktop_runtime.rs");
const hostConfig = read("src-tauri/src/config.rs");
const hermesConfig = read("src-tauri/src/hermes_config.rs");
const opencodeConfig = read("src-tauri/src/opencode_config.rs");
const opencodeSessions = read(
  "src-tauri/src/session_manager/providers/opencode.rs",
);
const opencodeUsage = read("src-tauri/src/services/session_usage_opencode.rs");
const codexConfig = read("src-tauri/src/codex_config.rs");
const codexStateDb = read("src-tauri/src/codex_state_db.rs");
const commandHost = read("src-tauri/src/commands/misc.rs");
const claudeMcp = read("src-tauri/src/claude_mcp.rs");
const databaseBackup = read("src-tauri/src/database/backup.rs");
const syncProtocol = read("src-tauri/src/services/sync_protocol.rs");
const skillService = read("src-tauri/src/services/skill.rs");
const domainTest = read("src-tauri/tests/codex_desktop_domain.rs");
const ci = read(".github/workflows/ci.yml");
const releaseCheck = read("scripts/tasks/release-check.mjs");

function rustFilesUnder(relativeDirectory: string): string[] {
  const files: string[] = [];
  const pending = [relativeDirectory];
  while (pending.length > 0) {
    const current = pending.pop();
    if (current === undefined) break;
    for (const entry of fs.readdirSync(path.join(ROOT, current), {
      withFileTypes: true,
    })) {
      const relative = path.posix.join(current, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== "target") pending.push(relative);
      } else if (entry.isFile() && relative.endsWith(".rs")) {
        files.push(relative);
      }
    }
  }
  return files.sort();
}

describe("Codex Windows interactive-user contract", () => {
  it("uses the Shell process as the sole ordinary startup identity proof", () => {
    expect(startup).not.toContain("WTSQueryUserToken");
    expect(startup).toContain("GetShellWindow");
    expect(startup).toContain("GetWindowThreadProcessId");
    expect(startup).toContain("OpenProcessToken");
    expect(startup).toContain("TokenUser");
    expect(startup).toContain("TOKEN_DUPLICATE");
    expect(startup).toContain("CreateEnvironmentBlock");
    expect(startup).toContain("DestroyEnvironmentBlock");
    expect(startup).toContain("SHGetKnownFolderPath");
    expect(startup).toContain("FOLDERID_Profile");
    expect(startup).toContain("FOLDERID_LocalAppData");
    expect(startup).toContain("FOLDERID_RoamingAppData");

    expect(startupDomain).toContain("struct WindowsInteractiveUserContext");
    expect(startupDomain).toContain("process_session_id");
    expect(startupDomain).toContain("shell_session_id");
    expect(startupDomain).toContain("canonical_sid");
    expect(startupDomain).toContain("user_profile");
    expect(startupDomain).toContain("user_local_app_data");
    expect(startupDomain).toContain("user_roaming_app_data");
    expect(startupDomain).toContain("shell_command_paths");
    expect(startupDomain).toContain("WIN_INTERACTIVE_ENVIRONMENT_UNAVAILABLE");
    expect(startupDomain).not.toMatch(
      /derive\([^)]*(?:Serialize|Deserialize)[^)]*\)\s*\n[^\n]*WindowsInteractiveUserContext/,
    );
    expect(startupDomain).toContain(
      "elevated_bob_is_allowed_while_shell_alice_remains_authority",
    );
    expect(startupDomain).toContain("process_session_id != shell_session_id");
    expect(startupDomain).not.toContain("early_windows_startup_gate");
    expect(startup).not.toContain("FOLDERID_ProgramData");
  });

  it("keeps the explicit-SID Main query and removes main-process staging", () => {
    expect(
      deployment.match(/FindPackagesByUserSecurityIdWithPackageTypes/g),
    ).toHaveLength(1);
    expect(deployment.match(/PackageTypes::Main/g)).toHaveLength(1);
    expect(deployment).not.toContain(".FindPackages()");
    expect(deployment).not.toContain("FindPackagesWithPackageTypes");
    expect(deployment).not.toContain("StagePackageByUriAsync");
    expect(deployment).not.toContain("ProvisionPackageForAllUsersAsync");
    expect(deployment).not.toContain("AddPackageByUriAsync");
    expect(adapter).not.toContain("AddPackageByUriAsync");
    expect(deployment).not.toMatch(/deploy_current_user\s*\(/);

    const ordinaryFacade = deployment.match(
      /trait WindowsPackageManager[\s\S]*?\n}/,
    )?.[0];
    expect(ordinaryFacade).toBeDefined();
    expect(ordinaryFacade).not.toMatch(/all[_-]?users|FindPackages/);
  });

  it("isolates CommonApplicationData to the one-shot Codex package bridge", () => {
    const programDataOwners = [
      ...rustFilesUnder("src-tauri/src"),
      ...rustFilesUnder("src-tauri/user-helper/src"),
    ].filter((file) => read(file).includes("FOLDERID_ProgramData"));

    expect(programDataOwners).toEqual([
      "src-tauri/src/codex_desktop/platform/windows/package_bridge.rs",
      "src-tauri/user-helper/src/windows.rs",
    ]);
    expect(startup).not.toContain("FOLDERID_ProgramData");

    const packageBridgeRoot =
      "FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}";
    const bridgeSources = `${packageBridge}\n${userHelperRuntime}\n${userHelperLayout}`;
    expect(bridgeSources.split(packageBridgeRoot)).toHaveLength(2);
    for (const source of [packageBridge, userHelperRuntime]) {
      expect(source).toContain("SHGetKnownFolderPath");
      expect(source).toContain("FOLDERID_ProgramData");
      expect(source).not.toMatch(
        /std::env::(?:var|var_os|current_dir|temp_dir)|C:\\+ProgramData/iu,
      );
      expect(source).not.toMatch(
        /FyAgent[\\/]runtime|business-[*.]|HMAC|capability pipe|runtime lease/iu,
      );
    }

    for (const forbiddenScopeApi of [
      "StagePackageByUriAsync",
      "StagePackageAsync",
      "RegisterPackagesByFullNameAsync",
      "ProvisionPackageForAllUsersAsync",
      "FindPackages()",
    ]) {
      expect(
        `${adapter}\n${deployment}\n${helper}\n${packageBridge}\n${userHelperRuntime}`,
      ).not.toContain(forbiddenScopeApi);
    }
  });

  it("threads the frozen context through helper orchestration, inventory, runtime, and launch", () => {
    expect(adapter).toContain("Arc<InteractiveUserContext>");
    expect(deployment).toContain("InteractiveUserContext");
    expect(deployment).toMatch(/packages_for_user\s*\(/);
    expect(deployment).toMatch(/launch_aumid\s*\(/);
    expect(adapter).toContain("WindowsContextRevalidator");
    expect(adapter).toContain("WindowsUserHelperRunner");
    expect(adapter).toContain("require_current_context");
    expect(helper).toContain("SystemWindowsContextRevalidator");
    expect(helper).toContain("revalidate_interactive_user_context(context)");
    expect(deployment).toContain(
      "revalidate_interactive_user_context(context)",
    );
    expect(runtime).toContain("OpenProcessToken");
    expect(runtime).toContain("TokenUser");
    expect(runtime).toContain("revalidate_interactive_user_context(context)");
    expect(runtime).toContain(
      "user_sid_matches_context(context, process_sid.as_deref())",
    );
    expect(startup).toMatch(
      /pub\(super\) fn revalidate_interactive_user_context\([\s\S]{0,800}probe_identity\(false\)/,
    );
    const revalidation = startup.slice(
      startup.indexOf("pub(super) fn revalidate_interactive_user_context"),
      startup.indexOf("pub(super) fn runtime_privilege_status"),
    );
    expect(revalidation).not.toContain("shell_command_paths");
  });

  it("stages Windows jobs only below the frozen executable install root", () => {
    expect(desktopRuntime).toContain("JobTempRoot::for_current_process()");
    expect(tempRoot).toContain("CURRENT_EXECUTABLE_INSTALL_ROOT");
    expect(tempRoot).toContain("CurrentExecutableInstallRoot");
    expect(tempRoot).toMatch(
      /#\[cfg\(target_os = "windows"\)\]\s+\{\s+Self::CurrentExecutableInstallRoot\s+\}/u,
    );
    expect(tempRoot).toMatch(
      /#\[cfg\(target_os = "macos"\)\]\s+\{\s+Self::Explicit\(JobTempDir::system_root\(\)\)\s+\}/u,
    );
    expect(tempRoot).toMatch(
      /#\[cfg\(target_os = "macos"\)\]\s+pub\(crate\) fn system_root\(\)[\s\S]+?std::env::temp_dir\(\)/u,
    );

    const windowsJob = tempRoot.slice(
      tempRoot.indexOf("fn create_current_executable_job"),
      tempRoot.indexOf("\nfn trusted_ancestors_for_existing_root"),
    );
    for (const invariant of [
      "CanonicalJobId::parse(job_id)",
      "frozen.revalidate()",
      "derive_install_layout",
      "ensure_current_executable_staging_root()",
      "ArtifactPolicy::WindowsMsixOnly",
      "relative_installer != helper_relative",
      "Some(INSTALLER_FILE_NAME)",
    ]) {
      expect(windowsJob).toContain(invariant);
    }
    expect(desktopRuntime).not.toContain("std::env::temp_dir");
    expect(adapter).not.toContain('PathBuf::from("C:\\\\")');
  });

  it("initializes Shell paths before user state and bounds plugin activation", () => {
    const main = read("src-tauri/src/main.rs");
    const host = read("src-tauri/src/lib.rs");
    const appStore = read("src-tauri/src/app_store.rs");
    const appStoreProduction = appStore.slice(
      0,
      appStore.indexOf("#[cfg(test)]"),
    );
    const windowState = read("src-tauri/src/windows_window_state.rs");

    expect(main.indexOf("initialize_windows_user_context()")).toBeGreaterThan(
      -1,
    );
    expect(main).not.toContain("maybe_run_codex_desktop_headless");
    expect(host).toContain("normalize_single_instance_args(args)");
    expect(startupDomain).toContain("MAX_SINGLE_INSTANCE_ARGUMENTS: usize = 8");
    expect(startupDomain).toContain(
      "MAX_SINGLE_INSTANCE_ARGUMENT_BYTES: usize = 64 * 1024",
    );
    expect(startupDomain).toContain(
      "MAX_SINGLE_INSTANCE_JSON_BYTES: usize = 73_712",
    );
    expect(startupDomain).toContain("struct SingleInstanceEnvelope");
    expect(startupDomain).toContain("version: 1, args");
    expect(startupDomain).toContain("serialized_single_instance_envelope_size");
    expect(host).toContain('.find(|window| window.label == "main")');
    expect(host).toContain("window.create = false");
    expect(host).toContain("WebviewWindowBuilder::from_config");
    expect(host).toContain(
      ".data_directory(crate::windows_runtime::webview_user_data_dir",
    );
    expect(hostConfig).toContain("pub(crate) fn read_bounded_file");
    expect(appStore).toContain(
      "WINDOWS_APP_PATHS_STORE_MAX_BYTES: usize = 64 * 1024",
    );
    expect(appStoreProduction).toContain("crate::config::read_bounded_file");
    expect(appStoreProduction).not.toContain("std::fs::read");
    expect(appStoreProduction).toContain("if encoded.len() > max_bytes");
    expect(appStoreProduction).toContain(
      "crate::config::atomic_write(store_path, &encoded)",
    );
    expect(windowState).toContain("tauri_window_state_path");
    expect(windowState).toContain("WINDOW_STATE_MAX_BYTES: usize = 256 * 1024");
    expect(windowState).toContain("crate::config::read_bounded_file");
    expect(windowState).not.toContain("std::fs::read");
    expect(windowState).not.toContain("app.path().app_config_dir");
  });

  it("does not consume elevated-process user path environment on Windows", () => {
    expect(hostConfig).toContain(
      '#[cfg(any(target_os = "macos", test, feature = "test-hooks"))]',
    );
    expect(hostConfig).toMatch(
      /#\[cfg\(target_os = "windows"\)\]\s+\{\s+crate::windows_runtime::user_home_dir\(\)\s+\}/,
    );
    expect(hermesConfig).toMatch(
      /#\[cfg\(target_os = "macos"\)\]\s+if let Some\(raw\) = std::env::var_os\("HERMES_HOME"\)/,
    );
    expect(opencodeConfig).toMatch(
      /#\[cfg\(target_os = "macos"\)\]\s+if let Ok\(custom_path\) = std::env::var\("OPENCODE_DB"\)/,
    );
    const macosDataHomeVariable = ["X", "DG_DATA_HOME"].join("");
    expect(opencodeConfig).toContain(
      `const OPENCODE_DATA_HOME_ENV: &str = "${macosDataHomeVariable}";`,
    );
    expect(opencodeConfig).toMatch(
      /#\[cfg\(target_os = "macos"\)\]\s+pub\(crate\) fn get_opencode_data_dir\(\) -> PathBuf \{[\s\S]*?std::env::var_os\(OPENCODE_DATA_HOME_ENV\)/,
    );
    expect(opencodeConfig).toMatch(
      /#\[cfg\(target_os = "windows"\)\]\s+pub\(crate\) fn get_opencode_data_dir\(\) -> PathBuf \{\s+resolve_opencode_data_dir\(&crate::config::get_home_dir\(\), None\)/,
    );
    expect(opencodeSessions).toContain(
      "crate::opencode_config::get_opencode_data_dir()",
    );
    expect(opencodeSessions).toContain(
      "crate::opencode_config::get_opencode_db_path()",
    );
    expect(opencodeUsage).toContain(
      "use crate::opencode_config::get_opencode_db_path;",
    );
    expect(codexStateDb).toMatch(
      /#\[cfg\(target_os = "windows"\)\]\s+fn sqlite_home_from_env\(\) -> Option<PathBuf> \{[\s\S]*?None\s+\}/,
    );

    const windowsCodexCandidates = codexConfig.slice(
      codexConfig.indexOf("#[cfg(windows)]\nfn push_env_codex_cli_candidates"),
      codexConfig.indexOf("\nfn codex_cli_candidates"),
    );
    expect(windowsCodexCandidates).toContain("safe_command_search_paths");
    expect(windowsCodexCandidates).not.toContain("std::env");
    expect(codexConfig).toMatch(
      /#\[cfg\(windows\)\]\s+const CODEX_CLI_FIXED_CANDIDATES: &\[&str\] = &\[\];/,
    );
    expect(codexConfig).toContain("codex_bundled_cli_allowed");
    expect(codexConfig).toContain(
      "formal_windows_build_never_runs_user_codex_cli_fallback",
    );
    const windowsCommandEnvironment = startupDomain.slice(
      startupDomain.indexOf("pub(crate) fn configure_shell_user_command"),
      startupDomain.indexOf("pub(crate) fn tauri_user_store_path"),
    );
    expect(windowsCommandEnvironment).toContain("command.env_clear()");
    expect(windowsCommandEnvironment).toContain(
      "shell_command_path_value_for_context",
    );
    expect(windowsCommandEnvironment).toContain('.env("USERPROFILE"');
    expect(windowsCommandEnvironment).toContain('.env("ComSpec"');
    expect(windowsCommandEnvironment).not.toContain("var_os");
    expect(codexConfig).toContain("configure_shell_user_command");
    expect(startup).toContain("GetDriveTypeW");
    expect(startup).toContain("DRIVE_FIXED");

    const windowsManagerPaths = commandHost.slice(
      commandHost.indexOf("fn extend_windows_cli_manager_search_paths"),
      commandHost.indexOf("/// OpenCode install.sh"),
    );
    expect(windowsManagerPaths).toContain("safe_command_search_paths");
    expect(windowsManagerPaths).not.toContain("std::env");
    expect(commandHost).toContain(
      '#[cfg(target_os = "macos")]\n    extend_from_cli_path_env',
    );
    expect(commandHost).toContain("configure_shell_user_command");
    expect(
      commandHost.match(/configure_shell_user_command/g)?.length ?? 0,
    ).toBeGreaterThanOrEqual(5);
    const windowsPathDefault = commandHost.slice(
      commandHost.indexOf(
        '#[cfg(target_os = "windows")]\nfn resolve_path_default',
      ),
      commandHost.indexOf("/// 枚举工具在系统中的所有安装"),
    );
    expect(windowsPathDefault).toContain("shell_command_search_paths");
    expect(windowsPathDefault).not.toContain("build_tool_search_paths");
    const sharedToolExecution = commandHost.slice(
      commandHost.indexOf(
        "pub(crate) fn run_detected_tool_command_with_timeout",
      ),
      commandHost.indexOf("fn run_windows_tool_command_capture"),
    );
    expect(sharedToolExecution).toContain(
      "detected_tool_execution_boundary_for",
    );
    expect(commandHost).toContain("detected_tool_execution_boundary_for(true)");
    expect(commandHost).not.toContain('Command::new("taskkill")');
    const productionCommandHost = commandHost.slice(
      0,
      commandHost.indexOf("#[cfg(test)]"),
    );
    expect(productionCommandHost).not.toContain(
      "C:\\\\Program Files\\\\nodejs",
    );
    expect(commandHost).toContain('system_executable_path("taskkill.exe")');
    expect(commandHost).toMatch(
      /system_executable_path\("taskkill\.exe"\)[\s\S]*configure_shell_user_command/,
    );
    expect(commandHost).toContain(".raw_arg(&command_line)");

    expect(claudeMcp).toMatch(
      /#\[cfg\(windows\)\]\s+let paths = crate::windows_runtime::shell_command_search_paths\(\)/,
    );
    expect(claudeMcp).toContain("command_path_validation_allowed");
    expect(claudeMcp).toContain(
      "formal_windows_command_validation_fails_before_path_access",
    );
    expect(
      claudeMcp.indexOf("command_path_validation_allowed(true"),
    ).toBeLessThan(claudeMcp.indexOf("Path::new(cmd).exists()"));
    expect(claudeMcp).toContain("is_local_command_path(Path::new(cmd))");
    expect(claudeMcp).toContain(
      'let exts: Vec<String> = ".COM;.EXE;.BAT;.CMD"',
    );
    expect(hostConfig).toContain("fn get_user_temp_dir() -> PathBuf");
    expect(commandHost).toContain("crate::config::get_user_temp_dir()");
    expect(databaseBackup).toContain("NamedTempFile::new_in(&temp_root)");
    expect(syncProtocol).toContain("tempdir_in(&temp_root)");
    expect(skillService).toContain("tempfile::tempdir_in(&temp_root)");
  });

  it("pins fixed Shell-user registry locations without following registry links", () => {
    expect(cargoManifest).toContain('winreg = "0.55"');
    expect(shellRegistry).toContain("enum ShellUserRegistryLocation");
    expect(shellRegistry).toContain("Environment");
    expect(shellRegistry).toContain("CurrentVersion");
    expect(shellRegistry).toContain("Run");
    expect(shellRegistry).toContain("REG_OPTION_OPEN_LINK");
    expect(shellRegistry).toContain('get_raw_value("SymbolicLinkValue")');
    expect(shellRegistry).toContain("REG_LINK");
    expect(shellRegistry).toContain("REG_CREATED_NEW_KEY");
    expect(shellRegistry).toContain("REG_OPENED_EXISTING_KEY");
    expect(shellRegistry).toContain("drop(created_handle)");
    expect(shellRegistry).toContain(
      "any_leaf_or_intermediate_symbolic_link_marker_rejects_the_traversal",
    );
    expect(shellRegistry).toContain(
      "isolated_native_open_link_accepts_normal_keys_and_rejects_link_components",
    );
    expect(shellRegistry).not.toContain("KEY_ALL_ACCESS");

    expect(startupDomain).toContain("mod registry;");
    expect(startupDomain).not.toContain("shell_user_registry_subkey");
    expect(autoLaunch).toContain("open_shell_user_run_update");
    expect(autoLaunch).not.toContain("HKEY_USERS");
    expect(envChecker).toContain("open_shell_user_environment_read");
    expect(envChecker).not.toContain("HKEY_USERS");
    expect(envManager).toContain("open_shell_user_environment_update");
    expect(envManager).toContain(
      "create_or_open_shell_user_environment_update",
    );
    expect(envManager).not.toContain("HKEY_USERS");
  });

  it("keeps Windows-only test targets connected to their compile-time dependencies", () => {
    const adapterTests = adapter.match(
      /#\[cfg\(test\)\]\s+mod tests \{[\s\S]*$/,
    )?.[0];
    expect(adapterTests).toBeDefined();
    expect(adapterTests).toContain("SuggestedAction::ResolvePathConflict");
    expect(adapterTests).toMatch(
      /error::\{[^}]*InstallerErrorCode[^}]*SuggestedAction[^}]*\}/,
    );
    expect(domainTest).toMatch(
      /#\[cfg\(target_os = "windows"\)\]\s+#\[allow\(dead_code, unused_imports, clippy::enum_variant_names\)\]\s+#\[path = "\.\.\/src\/windows_runtime\/mod\.rs"\]\s+mod windows_runtime;/,
    );
    expect(releaseCheck).toContain(
      '"tests/codexWindowsUserScopeContract.test.ts"',
    );
  });

  it("uses aligned backing storage for native SID structures", () => {
    for (const source of [startup, deployment, runtime]) {
      expect(source).not.toMatch(
        /vec!\[0_u8; required as usize\][\s\S]{0,500}cast::<TOKEN_USER>/,
      );
    }
    expect(startup).not.toContain("[0_u8; SECURITY_MAX_SID_SIZE as usize]");
  });

  it("runs the one native adapter smoke on both matching Windows architectures", () => {
    expect(ci).toContain("windows-2025");
    expect(ci).toContain("windows-11-arm");
    expect(ci).toContain("rust_host: x86_64-pc-windows-msvc");
    expect(ci).toContain("rust_host: aarch64-pc-windows-msvc");
    expect(ci).toContain("$env:RUNNER_ARCH -cne '${{ matrix.architecture }}'");
    expect(ci).toContain("--target '${{ matrix.rust_host }}'");
    expect(ci).toContain(
      "codex_desktop::platform::windows::deployment::tests::native_explicit_sid_main_query_smoke",
    );
    expect(ci).toContain("test result: ok\\. 1 passed; 0 failed");
    expect(deployment).toMatch(
      /fn native_explicit_sid_main_query_smoke\(\)[\s\S]*packages_for_user_sid_main\("not-a-windows-sid"\)/,
    );
  });
});
