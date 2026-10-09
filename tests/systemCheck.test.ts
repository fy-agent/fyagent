import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as systemCheckModule from "../scripts/tasks/system-check.mjs";

type SystemReport = {
  ok: boolean;
  platform: string;
  checks: Array<{ name: string; ok: boolean; hint?: string }>;
};
const inspect = systemCheckModule.inspect as (
  platform: string,
  probe: (command: string, args: string[]) => { status: number },
) => SystemReport;

const ROOT = path.resolve(__dirname, "..");
const SCRIPT = path.join(ROOT, "scripts", "tasks", "system-check.mjs");
const TOOLCHAIN_SCRIPT = path.join(
  ROOT,
  "scripts",
  "tasks",
  "toolchain-check.mjs",
);

function nodeScript(script: string, ...args: string[]) {
  return spawnSync(process.execPath, [script, ...args], {
    cwd: ROOT,
    encoding: "utf8",
  });
}

describe("read-only host prerequisite checks", () => {
  it.each(["darwin", "win32", "linux"])(
    "describes %s prerequisites without probing another host",
    (platform) => {
      const result = nodeScript(SCRIPT, "--describe-platform", platform);
      expect(result.status, result.stderr).toBe(0);
      const report = JSON.parse(result.stdout) as {
        platform: string;
        requirements: {
          commands: Array<[string, string[], string]>;
        };
      };
      expect(report.platform).toBe(platform);
      expect(report.requirements.commands.length).toBeGreaterThan(0);
      const forbiddenCommands = [
        "sudo",
        ["a", "pt"].join(""),
        "brew",
        "winget",
        "choco",
      ];
      for (const [command, args, hint] of report.requirements.commands) {
        expect(forbiddenCommands).not.toContain(command.toLowerCase());
        expect(args.join(" ")).not.toMatch(/(?:^|\s)(?:install|add)(?:\s|$)/i);
        expect(hint.length).toBeGreaterThan(0);
      }
    },
  );

  it("rejects an unsupported host without probing it", () => {
    const platform = "freebsd";
    const result = nodeScript(SCRIPT, "--describe-platform", platform);

    expect(result.status).toBe(2);
    expect(result.stdout).toBe("");
    expect(result.stderr).toContain(`Unknown platform: ${platform}`);
  });

  it("reports the current host as JSON and makes failures visible", () => {
    const result = spawnSync("mise", ["run", "system:check", "--json"], {
      cwd: ROOT,
      encoding: "utf8",
    });
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      platform: string;
      checks: Array<{ name: string; ok: boolean; hint?: string }>;
    };

    expect(report.platform).toBe(process.platform);
    expect(report.checks.length).toBeGreaterThan(0);
    expect(result.status === 0).toBe(report.ok);
    for (const check of report.checks.filter((entry) => !entry.ok)) {
      expect(check.hint).toBeTruthy();
    }
  });

  it.each(["darwin", "win32", "linux"])(
    "keeps reporting hints when every %s command probe is unavailable",
    (platform) => {
      // PATH does not control Windows' fixed-location vswhere lookup.
      const calls: Array<{ command: string; args: string[] }> = [];
      const report = inspect(platform, (command, args) => {
        calls.push({ command, args });
        return { status: 1 };
      });
      expect(report.ok).toBe(false);
      expect(report.platform).toBe(platform);
      expect(report.checks).toHaveLength(3);
      expect(calls).toHaveLength(report.checks.length);
      expect(
        calls.map(({ command, args }) => `${command} ${args.join(" ")}`),
      ).toEqual(report.checks.map((check) => check.name));
      expect(
        report.checks.every((check) => !check.ok && Boolean(check.hint)),
      ).toBe(true);
    },
  );

  it("continues probing after a missing tool and hints only failed checks", () => {
    const report = inspect("win32", (command) => ({
      status: command === "vswhere.exe" ? 1 : 0,
    }));
    expect(report.ok).toBe(false);
    expect(report.checks.map((check) => check.ok)).toEqual([true, false, true]);
    expect(report.checks[1].hint).toContain("Desktop development with C++");
    expect(report.checks[0].hint).toBeUndefined();
    expect(report.checks[2].hint).toBeUndefined();
  });

  it("contains no elevation or package-manager mutation command", () => {
    const source = fs.readFileSync(SCRIPT, "utf8");
    const forbiddenCommands = [
      "sudo",
      ["a", "pt"].join(""),
      ["a", "pt", "-get"].join(""),
      "brew",
      "winget",
      "choco",
    ];
    expect(source).not.toMatch(
      new RegExp(`run\\(["'](?:${forbiddenCommands.join("|")})["']`, "i"),
    );
    expect(source).not.toMatch(/exec(?:File)?Sync\(/);
  });

  it("normalizes native Windows separators before exact config-path comparison", () => {
    const backslashes = nodeScript(
      TOOLCHAIN_SCRIPT,
      "--normalize-path",
      "C:\\Repo\\FyAgent\\mise.toml",
    );
    const forwardSlashes = nodeScript(
      TOOLCHAIN_SCRIPT,
      "--normalize-path",
      "C:/Repo/FyAgent/mise.toml",
    );

    expect(backslashes.status, backslashes.stderr).toBe(0);
    expect(forwardSlashes.status, forwardSlashes.stderr).toBe(0);
    expect(backslashes.stdout.trim()).toBe(forwardSlashes.stdout.trim());
    const source = fs.readFileSync(TOOLCHAIN_SCRIPT, "utf8");
    expect(source).toContain("normalizeComparablePath(entry.path)");
    expect(source).not.toMatch(/entry\.path\.endsWith/);
    expect(source).toContain('capture("mise", ["which", "rustc"])');
    expect(source).toContain('capture("rustc", ["--print", "sysroot"])');
  });
});
