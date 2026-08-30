import { readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";

import { parse as parseToml } from "smol-toml";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const root = process.cwd();

type PermissionEntry = {
  identifier: string;
  commands: { allow: string[]; deny?: string[] };
};

function listTypeScriptFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const entryPath = join(directory, entry.name);
      if (entry.isDirectory()) return listTypeScriptFiles(entryPath);
      return [".ts", ".tsx"].includes(extname(entry.name)) ? [entryPath] : [];
    })
    .sort();
}

function rendererInvokeCommands(): {
  commands: Set<string>;
  dynamicInvokes: string[];
} {
  const commands = new Set<string>();
  const dynamicInvokes: string[] = [];

  for (const path of listTypeScriptFiles(
    join(root, "src/v2/shared/platform/tauri"),
  )) {
    const source = readFileSync(path, "utf8");
    const file = ts.createSourceFile(
      path,
      source,
      ts.ScriptTarget.Latest,
      true,
    );

    const visit = (node: ts.Node): void => {
      if (
        ts.isCallExpression(node) &&
        ts.isIdentifier(node.expression) &&
        node.expression.text === "invoke"
      ) {
        const command = node.arguments[0];
        if (command && ts.isStringLiteral(command)) {
          commands.add(command.text);
        } else {
          const position = file.getLineAndCharacterOfPosition(node.getStart());
          dynamicInvokes.push(
            `${path}:${position.line + 1}:${position.character + 1}`,
          );
        }
      }
      ts.forEachChild(node, visit);
    };
    visit(file);
  }
  return { commands, dynamicInvokes };
}

function registeredCommands(): Set<string> {
  const source = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
  const handler = source.slice(
    source.indexOf(".invoke_handler(tauri::generate_handler!["),
  );
  const commands = new Set(
    [...handler.matchAll(/^\s*commands::([A-Za-z0-9_]+),/gm)].map(
      (match) => match[1],
    ),
  );
  if (/^\s*update_tray_menu,/m.test(handler)) commands.add("update_tray_menu");
  return commands;
}

function activeAclCommands(): Set<string> {
  const capability = JSON.parse(
    readFileSync(join(root, "src-tauri/capabilities/default.json"), "utf8"),
  ) as {
    windows: string[];
    remote?: unknown;
    permissions: string[];
  };
  expect(capability.windows).toEqual(["main"]);
  expect(capability.remote).toBeUndefined();

  const activeAppPermissions = new Set(
    capability.permissions.filter((identifier) => !identifier.includes(":")),
  );
  const definitions = new Map<string, PermissionEntry>();
  const permissionsDirectory = join(root, "src-tauri/permissions");
  for (const file of readdirSync(permissionsDirectory).filter((name) =>
    name.endsWith(".toml"),
  )) {
    const manifest = parseToml(
      readFileSync(join(permissionsDirectory, file), "utf8"),
    ) as { permission?: PermissionEntry[] };
    for (const permission of manifest.permission ?? []) {
      expect(definitions.has(permission.identifier)).toBe(false);
      definitions.set(permission.identifier, permission);
    }
  }

  const allowed = new Set<string>();
  for (const identifier of activeAppPermissions) {
    const permission = definitions.get(identifier);
    expect(permission, `missing app permission ${identifier}`).toBeDefined();
    for (const command of permission?.commands.allow ?? [])
      allowed.add(command);
    for (const command of permission?.commands.deny ?? [])
      allowed.delete(command);
  }
  return allowed;
}

describe("V2 native ACL contract", () => {
  it("keeps every literal renderer invoke registered and allowed by the main capability", () => {
    const renderer = rendererInvokeCommands();
    const registered = registeredCommands();
    const allowed = activeAclCommands();

    expect(renderer.dynamicInvokes).toEqual([]);
    expect(renderer.commands.size).toBe(92);
    expect(
      [...renderer.commands].filter((command) => !registered.has(command)),
    ).toEqual([]);
    expect(
      [...renderer.commands].filter((command) => !allowed.has(command)),
    ).toEqual([]);
    expect([...registered].filter((command) => !allowed.has(command))).toEqual(
      [],
    );
    expect([...allowed].filter((command) => !registered.has(command))).toEqual(
      [],
    );
  });

  it("registers Change Plan, Agent action, and Agent auth commands", () => {
    const registered = registeredCommands();
    const expected = [
      "create_codex_provider_switch_plan",
      "create_codex_provider_upsert_plan",
      "create_workbuddy_save_plan",
      "apply_change_plan",
      "get_change_job",
      "list_recoverable_change_jobs",
      "get_agent_install_readiness",
      "get_agent_installation_inventory",
      "start_agent_action",
      "cancel_agent_action",
      "get_agent_action_job",
      "get_agent_auth_observation",
      "start_agent_auth_session",
      "get_agent_auth_session",
      "get_active_agent_auth_session",
      "stop_waiting_for_agent_auth",
    ];
    expect(expected.filter((command) => !registered.has(command))).toEqual([]);

    const forbidden = [...registered].filter((command) =>
      /(?:agent_install_(?:start|get_job|cancel|probe|doctor|helper)|fake_change|change_plan_cancel)/u.test(
        command,
      ),
    );
    expect(forbidden).toEqual([]);
  });
});
