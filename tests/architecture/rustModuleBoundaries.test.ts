import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(process.cwd());

function read(relativePath: string): string {
  return fs.readFileSync(path.join(repositoryRoot, relativePath), "utf8");
}

describe("Rust modular architecture boundaries", () => {
  it("keeps service implementation modules crate-scoped", () => {
    const services = read("src-tauri/src/services/mod.rs");
    const declarations = [
      ...services.matchAll(/^pub(?:\(crate\))? mod ([a-z0-9_]+);$/gmu),
    ];

    expect(declarations.length).toBeGreaterThan(30);
    expect(
      declarations
        .filter((match) => !match[0].startsWith("pub(crate) mod "))
        .map((match) => match[0]),
    ).toEqual([]);
  });

  it("keeps catch-all commands retired and system commands explicitly owned", () => {
    const commandModules = read("src-tauri/src/commands/mod.rs");
    const systemCommands = read("src-tauri/src/commands/system.rs");
    const toolingCommands = read("src-tauri/src/commands/tooling.rs");

    expect(commandModules).not.toMatch(/\bmod misc;/u);
    expect(commandModules).toContain("mod system;");
    expect(commandModules).toContain("mod tooling;");

    for (const command of [
      "open_external",
      "copy_text_to_clipboard",
      "is_portable_mode",
      "get_init_error",
      "get_migration_result",
      "get_skills_migration_result",
      "set_window_theme",
    ]) {
      expect(systemCommands).toMatch(
        new RegExp(`pub async fn ${command}\\b`, "u"),
      );
      expect(toolingCommands).not.toMatch(
        new RegExp(`pub async fn ${command}\\b`, "u"),
      );
    }
  });

  it("keeps Tooling transport limited to the reviewed Tauri command surface", () => {
    const toolingCommands = read("src-tauri/src/commands/tooling.rs");
    const toolingService = read("src-tauri/src/services/tooling.rs");
    const toolingLifecycle = read(
      "src-tauri/src/services/tooling/lifecycle.rs",
    );
    const toolingDiscovery = read(
      "src-tauri/src/services/tooling/discovery.rs",
    );
    const toolingTerminal = read("src-tauri/src/services/tooling/terminal.rs");
    const toolingVersions = read("src-tauri/src/services/tooling/versions.rs");
    const services = read("src-tauri/src/services/mod.rs");
    const commandNames = [
      ...toolingCommands.matchAll(
        /#\[tauri::command\]\s+pub async fn ([a-z0-9_]+)\b/gmu,
      ),
    ].map((match) => match[1]);

    expect(commandNames).toEqual([
      "get_tool_versions",
      "run_tool_lifecycle_action",
      "probe_tool_installations",
      "open_provider_terminal",
    ]);
    expect(services).toContain("pub(crate) mod tooling;");
    for (const implementationMarker of ["ELEVATED_WINDOWS_CLI_BOUNDARY_MESSAGE"]) {
      expect(toolingCommands).not.toContain(implementationMarker);
      expect(toolingService).toContain(implementationMarker);
    }
    for (const implementationMarker of [
      "build_tool_lifecycle_command",
      "enum ToolLifecycleAction",
    ]) {
      expect(toolingCommands).not.toContain(implementationMarker);
      expect(toolingLifecycle).toContain(implementationMarker);
    }
    expect(toolingService).not.toMatch(/\bfn build_tool_lifecycle_command\b/u);
    expect(toolingService).not.toMatch(/\benum ToolLifecycleAction\b/u);
    const grokRules = read("src-tauri/user-helper/src/grok.rs");
    expect(grokRules).toContain("fn powershell_encoded_command");
    expect(grokRules).toContain("GROK_NATIVE_WINDOWS_INSTALL_SCRIPT");
    expect(toolingLifecycle).toContain("fn grok_install_windows_command");
    for (const movedGrokRule of [
      "fn powershell_encoded_command",
      "GROK_INSTALL_WINDOWS_SCRIPT",
      "GROK_NATIVE_WINDOWS_INSTALL_SCRIPT",
    ]) {
      expect(toolingCommands).not.toContain(movedGrokRule);
      expect(toolingLifecycle).not.toContain(movedGrokRule);
      expect(toolingService).not.toContain(movedGrokRule);
    }
    expect(toolingService).toContain("mod grok_npm;");
    expect(toolingService).toMatch(
      /#\[cfg\(target_os = "macos"\)\]\s+use lifecycle::GROK_INSTALL_UNIX;/u,
    );
    for (const implementationMarker of [
      "fetch_npm_latest_for_tool",
      "pick_latest_version",
    ]) {
      expect(toolingCommands).not.toContain(implementationMarker);
      expect(toolingVersions).toContain(implementationMarker);
    }
    expect(toolingService).not.toMatch(/\bfn fetch_npm_latest_for_tool\b/u);
    expect(toolingService).not.toMatch(/\bfn pick_latest_version\b/u);
    for (const implementationMarker of [
      "struct ToolInstallationReport",
      "run_detected_tool_command_with_timeout",
    ]) {
      expect(toolingCommands).not.toContain(implementationMarker);
      expect(toolingDiscovery).toContain(implementationMarker);
    }
    expect(toolingService).not.toMatch(/\bstruct ToolInstallationReport\b/u);
    expect(toolingService).not.toMatch(
      /\bfn run_detected_tool_command_with_timeout\b/u,
    );
    for (const implementationMarker of [
      "extract_env_vars_from_config",
      "launch_terminal_running",
    ]) {
      expect(toolingCommands).not.toContain(implementationMarker);
      expect(toolingTerminal).toContain(implementationMarker);
    }
    expect(toolingService).not.toMatch(/\bfn extract_env_vars_from_config\b/u);
    expect(toolingService).not.toMatch(/\bfn launch_terminal_running\b/u);
    expect(toolingService).toContain("mod discovery;");
    expect(toolingService).toContain("mod lifecycle;");
    expect(toolingService).toContain("mod terminal;");
    expect(toolingService).toContain("mod versions;");
    expect(toolingService).not.toMatch(
      /pub(?:\(crate\))? mod (?:discovery|lifecycle|terminal|versions);/u,
    );
  });

  it("keeps extracted backend subdomains private behind their owning facades", () => {
    const provider = read("src-tauri/src/services/provider/mod.rs");
    const skill = read("src-tauri/src/services/skill.rs");
    const proxy = read("src-tauri/src/services/proxy.rs");
    const codex = read("src-tauri/src/codex_config.rs");
    const codexCatalog = read("src-tauri/src/codex_config/catalog.rs");

    expect(provider).toContain("mod common_config;");
    expect(provider).toContain("mod universal;");
    expect(skill).toContain("mod assignment;");
    expect(skill).toContain("mod discovery;");
    expect(skill).toContain("mod marketplace;");
    expect(skill).toContain("mod migration;");
    expect(skill).toContain("mod repository;");
    expect(proxy).toContain("mod takeover;");
    expect(codex).toContain("mod auth;");
    expect(codex).toContain("mod catalog;");
    expect(codex).toContain("mod features;");
    expect(codex).toContain("mod storage;");
    expect(codexCatalog).toContain("CODEX_MODEL_CATALOG_TEMPLATE_CACHE");
    expect(codexCatalog).toContain("fn codex_model_catalog_from_settings");
    expect(codexCatalog).toContain("fn prepare_codex_config_text_with_model_catalog");
    expect(codex).not.toMatch(/\bfn codex_model_catalog_from_settings\b/u);
    expect(codex).not.toMatch(
      /\bfn prepare_codex_config_text_with_model_catalog\b/u,
    );

    for (const source of [provider, skill, proxy, codex]) {
      expect(source).not.toMatch(
        /pub(?:\(crate\))? mod (?:common_config|universal|assignment|discovery|marketplace|migration|repository|takeover|auth|catalog|features|storage);/u,
      );
    }
  });
});
