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
    for (const implementationMarker of ["ELEVATED_WINDOWS_CLI_BOUNDARY_MESSAGE", "launch_terminal_running"]) {
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
    for (const implementationMarker of [
      "fetch_npm_latest_for_tool",
      "pick_latest_version",
    ]) {
      expect(toolingCommands).not.toContain(implementationMarker);
      expect(toolingVersions).toContain(implementationMarker);
    }
    expect(toolingService).not.toMatch(/\bfn fetch_npm_latest_for_tool\b/u);
    expect(toolingService).not.toMatch(/\bfn pick_latest_version\b/u);
    expect(toolingService).toContain("mod lifecycle;");
    expect(toolingService).toContain("mod versions;");
    expect(toolingService).not.toMatch(/pub(?:\(crate\))? mod (?:lifecycle|versions);/u);
  });

  it("keeps extracted backend subdomains private behind their owning facades", () => {
    const provider = read("src-tauri/src/services/provider/mod.rs");
    const skill = read("src-tauri/src/services/skill.rs");
    const proxy = read("src-tauri/src/services/proxy.rs");
    const codex = read("src-tauri/src/codex_config.rs");

    expect(provider).toContain("mod common_config;");
    expect(provider).toContain("mod universal;");
    expect(skill).toContain("mod discovery;");
    expect(skill).toContain("mod marketplace;");
    expect(skill).toContain("mod migration;");
    expect(skill).toContain("mod repository;");
    expect(proxy).toContain("mod takeover;");
    expect(codex).toContain("mod auth;");
    expect(codex).toContain("mod catalog;");
    expect(codex).toContain("mod features;");
    expect(codex).toContain("mod storage;");

    for (const source of [provider, skill, proxy, codex]) {
      expect(source).not.toMatch(
        /pub(?:\(crate\))? mod (?:common_config|universal|discovery|marketplace|migration|repository|takeover|auth|catalog|features|storage);/u,
      );
    }
  });
});
