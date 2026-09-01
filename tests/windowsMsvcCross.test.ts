import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as taskLibModule from "../scripts/tasks/lib.mjs";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as crossModule from "../scripts/tasks/windows-msvc-cross.mjs";

const ROOT = path.resolve(__dirname, "..");
const SAFE_ENVIRONMENT = Object.freeze({
  CARGO_HOME: path.join(ROOT, "target", "fixture-empty-cargo-home"),
});

type RunResult = { status: number; stdout: string; stderr: string };
type RunCall = { command: string; args: string[] };

const readToml = taskLibModule.readToml as (
  relativePath: string,
) => Record<string, Record<string, unknown>>;

function passingRunner(calls: RunCall[]) {
  return (command: string, args: string[]): RunResult => {
    calls.push({ command, args });
    const key = `${command} ${args.join(" ")}`;
    if (key === "cargo xwin --version") {
      return { status: 0, stdout: "cargo-xwin-xwin 0.23.1", stderr: "" };
    }
    if (key === "cargo clippy --version") {
      return { status: 0, stdout: "clippy 0.1.97", stderr: "" };
    }
    if (key === "rustup target list --installed") {
      return {
        status: 0,
        stdout: "aarch64-apple-darwin\nx86_64-pc-windows-msvc\n",
        stderr: "",
      };
    }
    return { status: 0, stdout: `${command} fixture`, stderr: "" };
  };
}

describe("optional macOS Windows MSVC cross diagnostic", () => {
  it("admits only the explicit macOS host matrix", () => {
    expect(crossModule.WINDOWS_MSVC_CROSS_HOST_TARGETS).toEqual({
      "darwin-x64": "x86_64-pc-windows-msvc",
      "darwin-arm64": "x86_64-pc-windows-msvc",
    });
    expect(crossModule.expectedWindowsMsvcCrossTarget("darwin", "x64")).toBe(
      crossModule.WINDOWS_MSVC_CROSS_TARGET,
    );
    expect(crossModule.expectedWindowsMsvcCrossTarget("darwin", "arm64")).toBe(
      crossModule.WINDOWS_MSVC_CROSS_TARGET,
    );
    expect(() =>
      crossModule.expectedWindowsMsvcCrossTarget("freebsd", "x64"),
    ).toThrow("require macOS x64/arm64");
  });

  it("checks every reviewed prerequisite and exact cargo-xwin version", () => {
    const calls: RunCall[] = [];
    const report = crossModule.inspectWindowsMsvcCross({
      platform: "darwin",
      environment: SAFE_ENVIRONMENT,
      runCommand: passingRunner(calls),
    }) as {
      ok: boolean;
      target: string;
      checks: Array<{ id: string; ok: boolean }>;
    };

    expect(report.ok).toBe(true);
    expect(report.target).toBe("x86_64-pc-windows-msvc");
    expect(report.checks.map(({ id }) => id)).toEqual([
      "cargo-xwin",
      "clippy",
      "rust-target",
      "clang-cl",
      "lld-link",
      "llvm-lib",
      "cmake",
      "ninja",
    ]);
    expect(report.checks.every(({ ok }) => ok)).toBe(true);
    expect(calls).toHaveLength(8);
    expect(crossModule.parseCargoXwinVersion("cargo-xwin 0.23.1")).toBe(
      "0.23.1",
    );
    expect(crossModule.parseCargoXwinVersion("cargo-xwin-xwin 0.23.1")).toBe(
      "0.23.1",
    );
    expect(() =>
      crossModule.parseCargoXwinVersion("cargo-xwin 0.23.1 extra"),
    ).toThrow("exactly one stable version");
  });

  it("fails before probing on an unsupported host or caller override", () => {
    let calls = 0;
    const forbiddenRunner = () => {
      calls += 1;
      throw new Error("a rejected preflight must not launch a child process");
    };

    const unsupported = crossModule.inspectWindowsMsvcCross({
      platform: "freebsd",
      environment: SAFE_ENVIRONMENT,
      runCommand: forbiddenRunner,
    }) as { ok: boolean; checks: Array<{ id: string }> };
    expect(unsupported.ok).toBe(false);
    expect(unsupported.checks.map(({ id }) => id)).toEqual(["supported-host"]);

    const overridden = crossModule.inspectWindowsMsvcCross({
      platform: "darwin",
      environment: { ...SAFE_ENVIRONMENT, XWIN_VERSION: "18" },
      runCommand: forbiddenRunner,
    }) as { ok: boolean; checks: Array<{ id: string; hint?: string }> };
    expect(overridden.ok).toBe(false);
    expect(overridden.checks[0]).toMatchObject({ id: "caller-environment" });
    expect(overridden.checks[0]?.hint).toContain(
      "XWIN_VERSION must not be set",
    );
    expect(calls).toBe(0);
  });

  it("reports every missing tool instead of failing at the first native dependency", () => {
    const report = crossModule.inspectWindowsMsvcCross({
      platform: "darwin",
      environment: SAFE_ENVIRONMENT,
      runCommand: () => ({
        status: 1,
        stdout: "",
        stderr: "fixture missing",
      }),
    }) as {
      ok: boolean;
      checks: Array<{ ok: boolean; hint?: string; detail?: string }>;
    };

    expect(report.ok).toBe(false);
    expect(report.checks).toHaveLength(8);
    expect(
      report.checks.every(
        ({ ok, hint, detail }) => !ok && Boolean(hint) && Boolean(detail),
      ),
    ).toBe(true);
  });

  it("rejects an unreviewed cargo-xwin release", () => {
    const calls: RunCall[] = [];
    const runner = passingRunner(calls);
    const report = crossModule.inspectWindowsMsvcCross({
      platform: "darwin",
      environment: SAFE_ENVIRONMENT,
      runCommand(command: string, args: string[]) {
        if (`${command} ${args.join(" ")}` === "cargo xwin --version") {
          return { status: 0, stdout: "cargo-xwin-xwin 0.24.0", stderr: "" };
        }
        return runner(command, args);
      },
    }) as {
      ok: boolean;
      checks: Array<{ id: string; ok: boolean; hint?: string }>;
    };

    expect(report.ok).toBe(false);
    expect(report.checks.find(({ id }) => id === "cargo-xwin")).toMatchObject({
      ok: false,
      hint: "cargo-xwin must be exactly 0.23.1; found 0.24.0.",
    });
  });

  it("owns a fixed x64 clang-cl Clippy plan with warnings denied", () => {
    expect(
      crossModule.planWindowsMsvcCrossClippy({
        platform: "darwin",
        environment: SAFE_ENVIRONMENT,
      }),
    ).toEqual({
      command: "cargo",
      args: [
        "xwin",
        "clippy",
        "--cross-compiler",
        "clang-cl",
        "--xwin-version",
        "17",
        "--target",
        "x86_64-pc-windows-msvc",
        "--workspace",
        "--all-targets",
        "--locked",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--",
        "-D",
        "warnings",
      ],
    });
    expect(() =>
      crossModule.planWindowsMsvcCrossClippy({
        platform: "darwin",
        environment: SAFE_ENVIRONMENT,
        forwardedArguments: ["--release"],
      }),
    ).toThrow("does not accept forwarded arguments");
    expect(() =>
      crossModule.planWindowsMsvcCrossClippy({
        platform: "darwin",
        environment: {
          ...SAFE_ENVIRONMENT,
          CARGO_BUILD_TARGET: "x86_64-pc-windows-msvc",
        },
      }),
    ).toThrow("CARGO_BUILD_TARGET must not be set");
    expect(() =>
      crossModule.planWindowsMsvcCrossClippy({
        platform: "darwin",
        environment: { ...SAFE_ENVIRONMENT, RUSTFLAGS: "-A warnings" },
      }),
    ).toThrow("RUSTFLAGS must not be set");
  });

  it("keeps the diagnostic explicit and outside the default check DAG", () => {
    const core = readToml(".mise/tasks/core.toml");
    const rust = readToml(".mise/tasks/rust.toml");
    expect(core["system:check:windows-msvc-cross"]).toMatchObject({
      env: { FYAGENT_TASK_EFFECT: "read-only" },
      run: "node scripts/tasks/windows-msvc-cross.mjs check",
    });
    expect(core["system:check:windows-msvc-cross:advisory"]).toMatchObject({
      env: { FYAGENT_TASK_EFFECT: "read-only" },
      run: "node scripts/tasks/windows-msvc-cross.mjs advisory",
    });
    expect(rust["rust:clippy:windows-msvc-cross"]).toMatchObject({
      env: { FYAGENT_TASK_EFFECT: "dependency-environment" },
      confirm: { default: "no" },
      run: "node scripts/tasks/windows-msvc-cross.mjs clippy",
    });
    expect(JSON.stringify(core.bootstrap)).toContain(
      "system:check:windows-msvc-cross:advisory",
    );
    expect(JSON.stringify(core.check)).not.toContain("windows-msvc-cross");
    expect(JSON.stringify(core["check:backend"])).not.toContain(
      "windows-msvc-cross",
    );
  });

  it("keeps the preflight read-only and reports the real host as JSON", () => {
    const source = fs.readFileSync(
      path.join(ROOT, "scripts", "tasks", "windows-msvc-cross.mjs"),
      "utf8",
    );
    expect(source).not.toMatch(/shell\s*:\s*true/u);
    expect(source).not.toMatch(
      /run(?:Command)?\s*\(\s*["'](?:brew|rustup|cargo)["']\s*,\s*\[\s*["'](?:install|target add|component add)/u,
    );

    const result = spawnSync(
      "mise",
      ["run", "system:check:windows-msvc-cross", "--json"],
      { cwd: ROOT, encoding: "utf8" },
    );
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      platform: string;
      checks: Array<{ ok: boolean; hint?: string }>;
    };
    expect(report.platform).toBe(process.platform);
    expect(report.checks.length).toBeGreaterThan(0);
    expect(result.status === 0).toBe(report.ok);
    expect(
      report.checks.filter(({ ok }) => !ok).every(({ hint }) => Boolean(hint)),
    ).toBe(true);
  });

  it("keeps bootstrap advisory even when the optional toolchain is incomplete", () => {
    const result = spawnSync(
      "mise",
      ["run", "system:check:windows-msvc-cross:advisory"],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(result.status).toBe(0);
    expect(result.stderr).toBe("");
    expect(result.stdout).toMatch(
      /Windows MSVC cross prerequisites|optional macOS-only diagnostic/u,
    );
  });

});
