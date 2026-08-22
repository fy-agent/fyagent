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
  });

  it("keeps extracted backend subdomains private behind their owning facades", () => {
    const provider = read("src-tauri/src/services/provider/mod.rs");
    const skill = read("src-tauri/src/services/skill.rs");
    const proxy = read("src-tauri/src/services/proxy.rs");
    const codex = read("src-tauri/src/codex_config.rs");

    expect(provider).toContain("mod universal;");
    expect(skill).toContain("mod discovery;");
    expect(proxy).toContain("mod takeover;");
    expect(codex).toContain("mod storage;");

    for (const source of [provider, skill, proxy, codex]) {
      expect(source).not.toMatch(
        /pub(?:\(crate\))? mod (?:universal|discovery|takeover|storage);/u,
      );
    }
  });
});
