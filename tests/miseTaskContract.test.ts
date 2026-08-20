import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as taskLibModule from "../scripts/tasks/lib.mjs";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as hostNativeModule from "../scripts/tasks/host-native.mjs";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as formatFilesModule from "../scripts/tasks/format-files.mjs";

const ROOT = path.resolve(__dirname, "..");
const FORMAT_FIXTURES = new Set<string>();

function createFormatFixture(prefix: string) {
  const fixture = fs.mkdtempSync(path.join(ROOT, `.${prefix}`));
  FORMAT_FIXTURES.add(fixture);
  return fixture;
}

process.once("exit", () => {
  for (const fixture of FORMAT_FIXTURES) {
    fs.rmSync(fixture, { recursive: true, force: true });
  }
});

type TaskDefinition = {
  confirm?: { default?: string; message?: string };
  env: { FYAGENT_TASK_EFFECT: string };
  interactive?: boolean;
  raw?: boolean;
  usage?: string;
};

type ContractModule = {
  PARAMETERIZED_TASKS: readonly string[];
  RAW_TASKS: readonly string[];
  loadTaskDefinitions(): Record<string, TaskDefinition>;
};

type LockAsset = {
  checksum: string;
  url: string;
};

const readToml = taskLibModule.readToml as (relativePath: string) => unknown;
const resolveTaskExecutable = taskLibModule.resolveTaskExecutable as (
  command: string,
  platform?: NodeJS.Platform,
) => string;

function mise(...args: string[]) {
  return spawnSync("mise", ["run", ...args], {
    cwd: ROOT,
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1" },
  });
}

function taskEnvironment(overrides: Record<string, string>) {
  const environment = { ...process.env };
  const controlled = new Set(
    [
      "CARGO_BUILD_TARGET",
      "TAURI_ENV_TARGET_TRIPLE",
      "RUSTC",
      "CARGO_BUILD_RUSTC",
      "RUSTC_WRAPPER",
      "CARGO_BUILD_RUSTC_WRAPPER",
      "RUSTC_WORKSPACE_WRAPPER",
      "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
      "RUSTDOC",
      "CARGO_BUILD_RUSTDOC",
      "RUSTFLAGS",
      "CARGO_BUILD_RUSTFLAGS",
      "CARGO_ENCODED_RUSTFLAGS",
      "RUSTDOCFLAGS",
      "CARGO_BUILD_RUSTDOCFLAGS",
      "CARGO_ENCODED_RUSTDOCFLAGS",
      "DYLD_INSERT_LIBRARIES",
      "DYLD_FORCE_FLAT_NAMESPACE",
      "DYLD_LIBRARY_PATH",
      "DYLD_FRAMEWORK_PATH",
      "DYLD_FALLBACK_LIBRARY_PATH",
      "NODE_OPTIONS",
      "NODE_PATH",
    ].map((name) => name.toUpperCase()),
  );
  for (const name of Object.keys(environment)) {
    const normalized = name.toUpperCase();
    if (
      controlled.has(normalized) ||
      /^CARGO_TARGET_.+_(?:RUNNER|LINKER|RUSTFLAGS|RUSTDOCFLAGS)$/.test(
        normalized,
      )
    ) {
      delete environment[name];
    }
  }
  return { ...environment, NO_COLOR: "1", ...overrides };
}

function foreignRustTarget(): string {
  const current = hostNativeModule.expectedRustTarget(
    process.platform,
    process.arch,
  ) as string;
  const foreign = Object.values(
    hostNativeModule.HOST_RUST_TARGETS as Record<string, string>,
  ).find((target) => target !== current);
  if (!foreign) throw new Error("No foreign Rust target fixture is available");
  return foreign;
}

function output(result: ReturnType<typeof mise>): string {
  return `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
}

function digest(relativePath: string): string {
  return createHash("sha256")
    .update(fs.readFileSync(path.join(ROOT, relativePath)))
    .digest("hex");
}

describe("canonical mise task API", () => {
  it("uses mise's native Windows pnpm executable without changing other commands", () => {
    expect(resolveTaskExecutable("pnpm", "win32")).toBe("pnpm.exe");
    expect(resolveTaskExecutable("pnpm", "darwin")).toBe("pnpm");
    expect(resolveTaskExecutable("pnpm", "linux")).toBe("pnpm");

    for (const command of ["npm", "npx", "pnpx", "node", "cargo"]) {
      expect(resolveTaskExecutable(command, "win32"), command).toBe(command);
    }
  });

  it("formats only reviewed repository files and preserves argv boundaries", () => {
    const fixture = createFormatFixture("format-files-test-");
    const relativeFixture = path.relative(ROOT, fixture);
    const spaced = path.join(relativeFixture, "with space.json");
    const unicode = path.join(relativeFixture, "配置.json");
    fs.writeFileSync(path.join(ROOT, spaced), "{}\n");
    fs.writeFileSync(path.join(ROOT, unicode), "{}\n");

    const calls: Array<{ command: string; args: string[] }> = [];
    try {
      formatFilesModule.formatFiles(
        [spaced, path.join(ROOT, unicode)],
        (command: string, args: string[]) => calls.push({ command, args }),
      );
      expect(calls).toEqual([
        {
          command: "pnpm",
          args: [
            "exec",
            "prettier",
            "--write",
            "--",
            spaced,
            path.join(ROOT, unicode),
          ],
        },
      ]);

      for (const invalid of [
        [],
        ["--config"],
        ["../outside.json"],
        [path.join(os.tmpdir(), "outside.json")],
        [relativeFixture],
      ]) {
        expect(
          () => formatFilesModule.validateFormatFiles(invalid),
          JSON.stringify(invalid),
        ).toThrow();
      }

      if (process.platform === "darwin") {
        const outside = path.join(os.tmpdir(), `fyagent-format-${process.pid}`);
        const link = path.join(fixture, "escape.json");
        fs.writeFileSync(outside, "{}\n");
        fs.symlinkSync(outside, link);
        expect(() =>
          formatFilesModule.validateFormatFiles([path.relative(ROOT, link)]),
        ).toThrow(/regular non-symlink file/);
        fs.rmSync(outside, { force: true });
      }
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("normalizes JSONL records before invoking Prettier and leaves all files unchanged on a parse failure", () => {
    const fixture = createFormatFixture("format-files-jsonl-");
    const relativeFixture = path.relative(ROOT, fixture);
    const first = path.join(relativeFixture, "first.JSONL");
    const invalid = path.join(relativeFixture, "second.jsonl");
    const ordinary = path.join(relativeFixture, "ordinary.json");
    const firstOriginal = ' { "first": true }\r\n';
    const invalidOriginal = '{"second":true}\nnot-json\n';
    fs.writeFileSync(path.join(ROOT, first), firstOriginal);
    fs.writeFileSync(path.join(ROOT, invalid), invalidOriginal);
    fs.writeFileSync(path.join(ROOT, ordinary), '{"ordinary":true}\n');

    const calls: Array<{ command: string; args: string[] }> = [];
    try {
      expect(() =>
        formatFilesModule.formatFiles(
          [ordinary, first, invalid],
          (command: string, args: string[]) => calls.push({ command, args }),
        ),
      ).toThrow(`Invalid JSONL record at ${invalid}:2`);
      expect(calls).toEqual([]);
      expect(fs.readFileSync(path.join(ROOT, first), "utf8")).toBe(
        firstOriginal,
      );
      expect(fs.readFileSync(path.join(ROOT, invalid), "utf8")).toBe(
        invalidOriginal,
      );
      expect(fs.readFileSync(path.join(ROOT, ordinary), "utf8")).toBe(
        '{"ordinary":true}\n',
      );
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("forwards whitespace and Unicode paths through the real format:files task", () => {
    const fixture = createFormatFixture("format-files-mise-");
    const first = path.join(fixture, "with space.json");
    const second = path.join(fixture, "配置.json");
    const jsonl = path.join(fixture, "Trellis 配置.jsonl");
    const secondJsonl = path.join(fixture, "第二个 Trellis.jsonl");
    fs.writeFileSync(first, '{"value":1}\n');
    fs.writeFileSync(second, '{"value":2}\n');
    fs.writeFileSync(
      jsonl,
      ' { "value": 9007199254740993, "duplicate": 1, "duplicate": 2, "escaped": "\\u0061", "negativeZero": -0, "nested": [ 1, 2 ] } \r\n\t\r\n',
    );
    fs.writeFileSync(
      secondJsonl,
      ' { "record": 1 } \r\n \t\r\n { "record": 2 } \r\n',
    );

    try {
      const result = mise(
        "format:files",
        "--",
        path.relative(ROOT, first),
        path.relative(ROOT, second),
        path.relative(ROOT, jsonl),
        path.relative(ROOT, secondJsonl),
      );
      expect(result.status, output(result)).toBe(0);
      expect(fs.readFileSync(first, "utf8")).toBe('{ "value": 1 }\n');
      expect(fs.readFileSync(second, "utf8")).toBe('{ "value": 2 }\n');
      expect(fs.readFileSync(jsonl, "utf8")).toBe(
        '{"value":9007199254740993,"duplicate":1,"duplicate":2,"escaped":"\\u0061","negativeZero":-0,"nested":[1,2]}\n\n',
      );
      expect(fs.readFileSync(secondJsonl, "utf8")).toBe(
        '{"record":1}\n\n{"record":2}\n',
      );
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("fails the real format:files task before changing any file when JSONL is invalid", () => {
    const fixture = createFormatFixture("format-files-invalid-");
    const ordinary = path.join(fixture, "ordinary file.json");
    const valid = path.join(fixture, "valid 配置.jsonl");
    const invalid = path.join(fixture, "invalid 配置.jsonl");
    const ordinaryOriginal = '{"ordinary":true}\n';
    const validOriginal = ' { "valid": true } \r\n';
    const invalidOriginal = '{"valid":true}\nnot-json\n';
    fs.writeFileSync(ordinary, ordinaryOriginal);
    fs.writeFileSync(valid, validOriginal);
    fs.writeFileSync(invalid, invalidOriginal);

    try {
      const result = mise(
        "format:files",
        "--",
        path.relative(ROOT, ordinary),
        path.relative(ROOT, valid),
        path.relative(ROOT, invalid),
      );
      expect(result.status, output(result)).not.toBe(0);
      expect(output(result)).toContain(
        `Invalid JSONL record at ${path.relative(ROOT, invalid)}:2`,
      );
      expect(fs.readFileSync(ordinary, "utf8")).toBe(ordinaryOriginal);
      expect(fs.readFileSync(valid, "utf8")).toBe(validOriginal);
      expect(fs.readFileSync(invalid, "utf8")).toBe(invalidOriginal);
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("locks native pnpm executables and checksums for both Windows architectures", () => {
    const lock = readToml("mise.lock") as {
      tools: {
        pnpm: Array<Record<string, LockAsset>>;
      };
    };
    expect(lock.tools.pnpm).toHaveLength(1);
    const [pnpm] = lock.tools.pnpm;
    for (const [platform, assetName] of [
      ["windows-x64", "pnpm-win-x64.exe"],
      ["windows-arm64", "pnpm-win-arm64.exe"],
    ] as const) {
      const asset = pnpm[`platforms.${platform}`];
      expect(asset.url.endsWith(`/${assetName}`), platform).toBe(true);
      expect(asset.checksum, platform).toMatch(/^sha256:[0-9a-f]{64}$/);
    }
  });

  it("rebuilds the full mise lock from an empty state with rollback protection", () => {
    const maintenance = fs.readFileSync(
      path.join(ROOT, "scripts", "tasks", "maintenance.mjs"),
      "utf8",
    );
    expect(maintenance).toContain(
      [
        'withFileRollback(["mise.lock"], () => {',
        '          writeFilesAtomically([["mise.lock", ""]]);',
        '          run("mise", ["lock", "--platform", platformArgument]);',
        '          run("node", ["scripts/tasks/lockfile-check.mjs"]);',
      ].join("\n"),
    );
  });

  it("loads a complete and extensible catalog with valid metadata", () => {
    const validation = spawnSync(
      "mise",
      ["tasks", "validate", "--errors-only"],
      {
        cwd: ROOT,
        encoding: "utf8",
      },
    );
    expect(validation.status).toBe(0);
    expect(output(validation)).toContain("task(s) validated successfully");

    const contract = spawnSync(
      process.execPath,
      ["scripts/tasks/task-contract-check.mjs"],
      { cwd: ROOT, encoding: "utf8" },
    );
    expect(contract.status, output(contract)).toBe(0);
    const report = JSON.parse(contract.stdout) as {
      ok: boolean;
      tasks: number;
      checkClosure: string[];
    };
    expect(report.ok).toBe(true);
    expect(report.tasks).toBeGreaterThanOrEqual(60);
    expect(report.checkClosure).toContain("check:contracts");
  });

  it("enforces usage, mutation, interactive, raw, and confirmation metadata", async () => {
    const contract = (await import(
      /* @vite-ignore */ pathToFileURL(
        path.join(ROOT, "scripts", "tasks", "task-contract-check.mjs"),
      ).href
    )) as ContractModule;
    const tasks = contract.loadTaskDefinitions();

    for (const name of contract.PARAMETERIZED_TASKS) {
      expect(tasks[name].usage?.trim(), name).toBeTruthy();
    }
    for (const [name, task] of Object.entries(tasks)) {
      const effect = task.env.FYAGENT_TASK_EFFECT;
      if (effect === "preview-by-default") {
        expect(task.usage, name).toContain('flag "--apply"');
      }
      expect(task.interactive === true, name).toBe(effect === "interactive");
    }
    expect(
      Object.entries(tasks)
        .filter(([, task]) => task.raw === true)
        .map(([name]) => name)
        .sort(),
    ).toEqual([...contract.RAW_TASKS].sort());
    expect(tasks["upstream:merge:abort"]).toMatchObject({
      confirm: { default: "no" },
      env: { FYAGENT_TASK_EFFECT: "git-state" },
    });
  });

  it("forwards a unit-test file filter through the real mise usage parser", () => {
    const result = mise("test:unit", "tests/developmentEnvironment.test.ts");
    expect(result.status, output(result)).toBe(0);
    expect(output(result)).toContain("developmentEnvironment.test.ts");
    expect(output(result)).not.toContain("miseTaskContract.test.ts");
  }, 60_000);

  it("routes the exact prearchive task only to the selected composite gate", async () => {
    const prearchive = (await import(
      /* @vite-ignore */ pathToFileURL(
        path.join(ROOT, "scripts", "tasks", "prearchive-check.mjs"),
      ).href
    )) as {
      resolvePrearchiveTarget: (mode: string) => string;
      runPrearchiveCheck: (
        mode: string,
        options: {
          activeTask: string;
          environment: NodeJS.ProcessEnv;
          validator: (value: string) => string;
          runner: (
            command: string,
            args: string[],
            options: { env: NodeJS.ProcessEnv },
          ) => unknown;
        },
      ) => unknown;
    };
    const activeTask = `.trellis/tasks/${[
      "08-12-remove",
      "lin",
      "ux-support",
    ].join("-")}`;
    const calls: Array<{
      command: string;
      args: string[];
      environment: NodeJS.ProcessEnv;
    }> = [];

    prearchive.runPrearchiveCheck("contracts", {
      activeTask,
      environment: { SAFE_PARENT: "1" },
      validator: (value) => {
        expect(value).toBe(activeTask);
        return value;
      },
      runner: (command, args, options) => {
        calls.push({ command, args, environment: options.env });
        return { status: 0 };
      },
    });

    expect(calls).toEqual([
      {
        command: "mise",
        args: ["run", "check:contracts"],
        environment: {
          SAFE_PARENT: "1",
          usage_exclude_active_task: "",
          FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK: activeTask,
        },
      },
    ]);
    expect(prearchive.resolvePrearchiveTarget("full")).toBe("check");
    expect(() => prearchive.resolvePrearchiveTarget("other")).toThrow();
    expect(() =>
      prearchive.runPrearchiveCheck("full", {
        activeTask,
        environment: { FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK: activeTask },
        validator: (value) => value,
        runner: () => ({ status: 0 }),
      }),
    ).toThrow(/must not be set by the caller/u);
  });

  it("forwards version and Python parameters while preview mode preserves files", () => {
    const guardedFiles = [
      "src-tauri/Cargo.toml",
      "src-tauri/Cargo.lock",
      "pyproject.toml",
      "uv.lock",
    ];
    const before = new Map(
      guardedFiles.map((relativePath) => [relativePath, digest(relativePath)]),
    );

    const version = mise("version:set", "0.3.0");
    expect(version.status, output(version)).toBe(0);
    expect(output(version)).toContain("0.3.0");
    expect(output(version)).toMatch(/would update|no files changed/i);

    const python = mise("python:add:dev", "httpx");
    expect(python.status, output(python)).toBe(0);
    expect(output(python)).toContain("httpx");
    expect(output(python)).toContain('"status": "preview"');

    for (const relativePath of guardedFiles) {
      expect(digest(relativePath), relativePath).toBe(before.get(relativePath));
    }
  });

  it("forwards upstream parameters before any Git mutation can run", () => {
    const upstreamTask = fs.readFileSync(
      path.join(ROOT, "scripts", "tasks", "upstream.mjs"),
      "utf8",
    );
    expect(upstreamTask).toContain(
      "const ORIGIN = /^https:\\/\\/github\\.com\\/fy-agent\\/fyagent(?:\\.git)?$/i;",
    );
    expect(upstreamTask).not.toContain(["NongHua123", "fyagent"].join("\\/"));

    const result = mise("upstream:merge:prepare", "not-a-release-tag");
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("Upstream tag must be exact vX.Y.Z");
    expect(output(result)).not.toContain("git merge");
  });

  it("forwards flags to the JSON environment report", () => {
    const result = mise("env:check", "--json");
    expect(result.status, output(result)).toBe(0);
    const report = JSON.parse(result.stdout) as {
      ok: boolean;
      checks: Array<{ name: string; ok: boolean }>;
    };
    expect(report.ok).toBe(true);
    expect(report.checks).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "Node ownership", ok: true }),
        expect.objectContaining({
          name: "Rust toolchain and components",
          ok: true,
        }),
      ]),
    );
  });

  it.each([
    ["split target", ["--", "--target", foreignRustTarget()]],
    ["equals target", ["--", `--target=${foreignRustTarget()}`]],
  ])("rejects %s injection before Cargo runs", (_label, args) => {
    const result = mise("rust:test", ...args);
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("Cargo options and targets are forbidden");
    expect(output(result)).not.toMatch(/Compiling|Finished.*test profile/);
  });

  it("plans the real wrapper runtime with rustc identity before launching the fixed host command", () => {
    const target = hostNativeModule.expectedRustTarget(
      process.platform,
      process.arch,
    ) as string;
    const rustcExecutable = "/verified/toolchain/bin/rustc";
    const rustdocExecutable = "/verified/toolchain/bin/rustdoc";
    const verbose = (tool: "rustc" | "rustdoc") =>
      `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1`;
    const calls: Array<{
      command: string;
      args: string[];
      environment?: Record<string, string>;
    }> = [];
    const captureCommand = (command: string, args: string[]) => {
      calls.push({ command, args });
      return command === rustcExecutable
        ? verbose("rustc")
        : verbose("rustdoc");
    };
    const runCommand = (
      command: string,
      args: string[],
      options: { env: Record<string, string> },
    ) => {
      calls.push({ command, args, environment: options.env });
    };
    const resolveToolCommand = ({ tool }: { tool: string }) =>
      tool === "rustc" ? rustcExecutable : rustdocExecutable;
    const nativeRunnerConfig = `target.${target}.runner=["/verified/node","/verified/host-native.mjs","native-runner","${target}"]`;
    const resolveRunner = () => nativeRunnerConfig;

    const tauri = hostNativeModule.executeTauriTask({
      operation: "build:debug",
      environment: {},
      platform: process.platform,
      architecture: process.arch,
      captureCommand,
      runCommand,
      resolveToolCommand,
      resolveMsvcEnvironment: () => ({}),
    }) as { command: string; args: string[]; target: string };
    expect(tauri).toMatchObject({
      command: "pnpm",
      args: ["tauri", "build", "--target", target, "--debug"],
      target,
    });
    expect(calls.map(({ command, args }) => ({ command, args }))).toEqual([
      { command: rustcExecutable, args: ["-vV"] },
      { command: rustdocExecutable, args: ["-vV"] },
      { command: "pnpm", args: tauri.args },
    ]);
    expect(calls[2].environment).toMatchObject({
      RUSTC: rustcExecutable,
      CARGO_BUILD_RUSTC: rustcExecutable,
      RUSTDOC: rustdocExecutable,
      CARGO_BUILD_RUSTDOC: rustdocExecutable,
      RUSTC_WRAPPER: "",
      RUSTC_WORKSPACE_WRAPPER: "",
      RUSTFLAGS: "",
      CARGO_ENCODED_RUSTFLAGS: "",
      RUSTDOCFLAGS: "",
      CARGO_ENCODED_RUSTDOCFLAGS: "",
    });

    calls.length = 0;
    const cargo = hostNativeModule.executeCargoTask({
      operation: "test",
      filters: ["settings"],
      environment: {},
      platform: process.platform,
      architecture: process.arch,
      captureCommand,
      runCommand,
      resolveToolCommand,
      resolveRunner,
      resolveMsvcEnvironment: () => ({}),
    }) as { command: string; args: string[]; target: string };
    expect(cargo.command).toBe("cargo");
    expect(cargo.args).toEqual([
      "--config",
      nativeRunnerConfig,
      "test",
      "--target",
      target,
      "--workspace",
      "--locked",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--features",
      "fyagent/test-hooks",
      "--no-fail-fast",
      "--",
      "settings",
    ]);
    const expectedCalls = [
      { command: rustcExecutable, args: ["-vV"] },
      { command: rustdocExecutable, args: ["-vV"] },
    ];
    switch (process.platform) {
      case "win32":
        expectedCalls.push({
          command: process.execPath,
          args: ["scripts/prepare-windows-user-helper.mjs"],
        });
        break;
      case "darwin":
        break;
      default:
        throw new Error(`Unsupported test host: ${process.platform}`);
    }
    expectedCalls.push({ command: "cargo", args: cargo.args });
    expect(calls.map(({ command, args }) => ({ command, args }))).toEqual(
      expectedCalls,
    );
    expect(calls.at(-1)?.environment).toMatchObject({
      RUSTC: rustcExecutable,
      RUSTDOC: rustdocExecutable,
    });
  });

  it.each(["check", "clippy", "test"])(
    "prepares the exact Windows helper once before rust:%s workspace Cargo",
    (operation) => {
      const platform: NodeJS.Platform = "win32";
      const architecture = "x64";
      const target = hostNativeModule.expectedRustTarget(
        platform,
        architecture,
      ) as string;
      const rustcExecutable = "C:\\verified\\toolchain\\rustc.exe";
      const rustdocExecutable = "C:\\verified\\toolchain\\rustdoc.exe";
      const nodeExecutable = "C:\\verified\\node.exe";
      const verbose = (tool: "rustc" | "rustdoc") =>
        `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1`;
      const calls: Array<{
        command: string;
        args: string[];
        environment?: Record<string, string>;
      }> = [];
      const sequence: string[] = [];

      const plan = hostNativeModule.executeCargoTask({
        operation,
        environment: {},
        platform,
        architecture,
        nodeExecutable,
        captureCommand: (command: string, args: string[]) => {
          sequence.push(
            command === rustcExecutable ? "probe:rustc" : "probe:rustdoc",
          );
          calls.push({ command, args });
          return command === rustcExecutable
            ? verbose("rustc")
            : verbose("rustdoc");
        },
        runCommand: (
          command: string,
          args: string[],
          options: { env: Record<string, string> },
        ) => {
          sequence.push(
            args[0] === "scripts/prepare-windows-user-helper.mjs"
              ? "run:helper"
              : "run:cargo",
          );
          calls.push({ command, args, environment: options.env });
        },
        resolveToolCommand: ({ tool }: { tool: string }) => {
          sequence.push(`resolve:${tool}`);
          return tool === "rustc" ? rustcExecutable : rustdocExecutable;
        },
        resolveRunner: () => {
          sequence.push("resolve:runner");
          return `target.${target}.runner=[${JSON.stringify(nodeExecutable)}]`;
        },
        validateCargoConfig: () => {
          sequence.push("validate:cargo-config");
        },
        resolveMsvcEnvironment: () => {
          sequence.push("resolve:msvc");
          return { INCLUDE: "C:\\vs\\include", LIB: "C:\\vs\\lib" };
        },
      }) as {
        command: string;
        args: string[];
        target: string;
        environment: Record<string, string>;
      };

      expect(calls).toHaveLength(4);
      expect(sequence).toEqual([
        "validate:cargo-config",
        "resolve:rustc",
        "resolve:rustdoc",
        "resolve:runner",
        "probe:rustc",
        "probe:rustdoc",
        "run:helper",
        "resolve:msvc",
        "run:cargo",
      ]);
      expect(calls.slice(0, 2)).toEqual([
        { command: rustcExecutable, args: ["-vV"] },
        { command: rustdocExecutable, args: ["-vV"] },
      ]);
      expect(calls[2]).toMatchObject({
        command: nodeExecutable,
        args: ["scripts/prepare-windows-user-helper.mjs"],
        environment: {
          RUSTC: rustcExecutable,
          RUSTDOC: rustdocExecutable,
          TAURI_ENV_TARGET_TRIPLE: target,
          TAURI_ENV_DEBUG: "true",
        },
      });
      expect(calls[3]).toEqual({
        command: "cargo",
        args: plan.args,
        environment: {
          ...plan.environment,
          INCLUDE: "C:\\vs\\include",
          LIB: "C:\\vs\\lib",
        },
      });
      expect(
        calls.filter(
          ({ args }) =>
            args.length === 1 &&
            args[0] === "scripts/prepare-windows-user-helper.mjs",
        ),
      ).toHaveLength(1);
    },
  );

  it.each([
    ["darwin", "x64"],
    ["darwin", "arm64"],
    ["linux", "x64"],
    ["linux", "arm64"],
  ] as const)(
    "does not prepare the Windows helper for %s/%s Rust tasks",
    (platform, architecture) => {
      const target = hostNativeModule.expectedRustTarget(
        platform,
        architecture,
      ) as string;
      const rustcExecutable = "/verified/toolchain/rustc";
      const rustdocExecutable = "/verified/toolchain/rustdoc";
      const calls: Array<{ command: string; args: string[] }> = [];

      hostNativeModule.executeCargoTask({
        operation: "check",
        environment: {},
        platform,
        architecture,
        nodeExecutable: "/verified/node",
        captureCommand: (command: string, args: string[]) => {
          calls.push({ command, args });
          const tool = command === rustcExecutable ? "rustc" : "rustdoc";
          return `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1`;
        },
        runCommand: (command: string, args: string[]) =>
          calls.push({ command, args }),
        resolveToolCommand: ({ tool }: { tool: string }) =>
          tool === "rustc" ? rustcExecutable : rustdocExecutable,
        resolveRunner: () => `target.${target}.runner=["/verified/node"]`,
      });

      expect(calls.map(({ command }) => command)).toEqual([
        rustcExecutable,
        rustdocExecutable,
        "cargo",
      ]);
      expect(calls.flatMap(({ args }) => args)).not.toContain(
        "scripts/prepare-windows-user-helper.mjs",
      );
    },
  );

  it("stops before Windows workspace Cargo when helper preparation fails", () => {
    const platform: NodeJS.Platform = "win32";
    const architecture = "arm64";
    const target = hostNativeModule.expectedRustTarget(
      platform,
      architecture,
    ) as string;
    const rustcExecutable = "C:\\verified\\toolchain\\rustc.exe";
    const rustdocExecutable = "C:\\verified\\toolchain\\rustdoc.exe";
    const nodeExecutable = "C:\\verified\\node.exe";
    const runCalls: Array<{ command: string; args: string[] }> = [];

    expect(() =>
      hostNativeModule.executeCargoTask({
        operation: "test",
        environment: {},
        platform,
        architecture,
        nodeExecutable,
        captureCommand: (command: string) => {
          const tool = command === rustcExecutable ? "rustc" : "rustdoc";
          return `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1`;
        },
        runCommand: (command: string, args: string[]) => {
          runCalls.push({ command, args });
          throw new Error("helper preparation failed");
        },
        resolveToolCommand: ({ tool }: { tool: string }) =>
          tool === "rustc" ? rustcExecutable : rustdocExecutable,
        resolveRunner: () =>
          `target.${target}.runner=[${JSON.stringify(nodeExecutable)}]`,
      }),
    ).toThrow("helper preparation failed");
    expect(runCalls).toEqual([
      {
        command: nodeExecutable,
        args: ["scripts/prepare-windows-user-helper.mjs"],
      },
    ]);
  });

  it("does not prepare the Windows helper before current-host toolchain validation", () => {
    let runCalls = 0;
    expect(() =>
      hostNativeModule.executeCargoTask({
        operation: "check",
        environment: {},
        platform: "win32",
        architecture: "x64",
        captureCommand: (command: string) => {
          const tool = command.toLowerCase().includes("rustdoc")
            ? "rustdoc"
            : "rustc";
          return `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: aarch64-pc-windows-msvc\nrelease: 1.97.1`;
        },
        runCommand: () => {
          runCalls += 1;
        },
        resolveToolCommand: ({ tool }: { tool: string }) =>
          `C:\\verified\\${tool}.exe`,
        resolveRunner: () => "verified-native-runner",
      }),
    ).toThrow(/does not match current host/);
    expect(runCalls).toBe(0);
  });

  it("requires the Windows helper preparer to use an absolute Node executable", () => {
    const target = "x86_64-pc-windows-msvc";
    let runCalls = 0;
    expect(() =>
      hostNativeModule.executeCargoTask({
        operation: "check",
        environment: {},
        platform: "win32",
        architecture: "x64",
        nodeExecutable: "node.exe",
        captureCommand: (command: string) => {
          const tool = command.toLowerCase().includes("rustdoc")
            ? "rustdoc"
            : "rustc";
          return `${tool} 1.97.1\ncommit-hash: verified-toolchain\nhost: ${target}\nrelease: 1.97.1`;
        },
        runCommand: () => {
          runCalls += 1;
        },
        resolveToolCommand: ({ tool }: { tool: string }) =>
          `C:\\verified\\${tool}.exe`,
        resolveRunner: () => "verified-native-runner",
      }),
    ).toThrow("canonical Node executable must be an absolute Windows path");
    expect(runCalls).toBe(0);
  });

  it.runIf(process.platform === "darwin")(
    "rejects effective Cargo toolchain config includes, cycles, and symlinks before tools start",
    () => {
      const fixture = fs.mkdtempSync(
        path.join(os.tmpdir(), "fyagent-cargo-config-contract-"),
      );
      const configDirectory = path.join(fixture, ".cargo");
      const configFile = path.join(configDirectory, "config.toml");
      const includedFile = path.join(configDirectory, "included.toml");
      const cargoHome = path.join(fixture, "cargo-home");
      fs.mkdirSync(configDirectory, { recursive: true });
      fs.mkdirSync(cargoHome, { recursive: true });

      try {
        fs.writeFileSync(
          configFile,
          'include = [{ path = "included.toml", optional = true }]\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).not.toThrow();

        fs.writeFileSync(
          includedFile,
          '[target.aarch64-apple-darwin]\nrunner = "/tmp/emulator"\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/target\.aarch64-apple-darwin\.runner is forbidden/);

        fs.writeFileSync(
          includedFile,
          '[target.aarch64-apple-darwin]\nlinker = "/tmp/linker"\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/target\.aarch64-apple-darwin\.linker is forbidden/);

        fs.writeFileSync(
          includedFile,
          '[target.aarch64-apple-darwin]\nrustflags = ["-C", "linker=/tmp/linker"]\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/target\.aarch64-apple-darwin\.rustflags is forbidden/);

        fs.writeFileSync(
          includedFile,
          '[env]\nnode_options = { value = "--require=/tmp/inject.js", force = true }\n',
        );
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/env\.node_options is forbidden/);

        fs.writeFileSync(includedFile, 'include = ["config.toml"]\n');
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/include cycle is forbidden/);

        fs.writeFileSync(includedFile, "[build]\nrustflags = []\n");
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/build\.rustflags is forbidden/);

        fs.rmSync(configFile);
        fs.symlinkSync(includedFile, configFile);
        expect(() =>
          hostNativeModule.assertNoCargoToolchainConfig({
            root: fixture,
            environment: { CARGO_HOME: cargoHome },
          }),
        ).toThrow(/regular non-symlink file/);
      } finally {
        fs.rmSync(fixture, { recursive: true, force: true });
      }
    },
  );

  it("validates Windows PE and macOS Mach-O test executables inside the current target directory", () => {
    const cases = [
      {
        platform: "win32",
        architecture: "x64",
        target: "x86_64-pc-windows-msvc",
        machineOffset: 68,
        validMachine: 0x8664,
        wrongMachine: 0xaa64,
        bytes() {
          const bytes = Buffer.alloc(128);
          bytes.write("MZ", 0, "ascii");
          bytes.writeUInt32LE(64, 0x3c);
          bytes.write("PE\0\0", 64, "binary");
          bytes.writeUInt16LE(this.validMachine, this.machineOffset);
          return bytes;
        },
      },
      {
        platform: "darwin",
        architecture: "arm64",
        target: "aarch64-apple-darwin",
        machineOffset: 4,
        validMachine: 0x0100000c,
        wrongMachine: 0x01000007,
        bytes() {
          const bytes = Buffer.alloc(64);
          bytes.writeUInt32BE(0xfeedfacf, 0);
          bytes.writeUInt32BE(this.validMachine, this.machineOffset);
          return bytes;
        },
      },
      {
        platform: "linux",
        architecture: "x64",
        target: "x86_64-unknown-linux-gnu",
        machineOffset: 18,
        validMachine: 62,
        wrongMachine: 183,
        bytes() {
          const bytes = Buffer.alloc(64);
          bytes[0] = 0x7f;
          bytes[1] = 0x45;
          bytes[2] = 0x4c;
          bytes[3] = 0x46;
          bytes[4] = 2;
          bytes[5] = 1;
          bytes.writeUInt16LE(this.validMachine, this.machineOffset);
          return bytes;
        },
      },
    ] as const;

    for (const fixtureCase of cases) {
      const targetDirectory = path.join(
        ROOT,
        "src-tauri",
        "target",
        fixtureCase.target,
      );
      const targetDirectoryExisted = fs.existsSync(targetDirectory);
      fs.mkdirSync(targetDirectory, { recursive: true });
      const fixtureDirectory = fs.mkdtempSync(
        path.join(targetDirectory, "fyagent-native-runner-contract-"),
      );

      try {
        const valid = path.join(fixtureDirectory, "valid-native-binary");
        fs.writeFileSync(valid, fixtureCase.bytes());
        expect(
          hostNativeModule.verifyNativeTestExecutable({
            target: fixtureCase.target,
            executable: valid,
            platform: fixtureCase.platform,
            architecture: fixtureCase.architecture,
            cwd: ROOT,
            root: ROOT,
          }),
        ).toBe(fs.realpathSync(valid));

        const malformed = path.join(fixtureDirectory, "malformed");
        fs.writeFileSync(malformed, "not a native executable");
        expect(() =>
          hostNativeModule.verifyNativeTestExecutable({
            target: fixtureCase.target,
            executable: malformed,
            platform: fixtureCase.platform,
            architecture: fixtureCase.architecture,
            cwd: ROOT,
            root: ROOT,
          }),
        ).toThrow(/must have|must be/);

        const wrongArchitecture = path.join(
          fixtureDirectory,
          "wrong-architecture",
        );
        const wrongBytes = fixtureCase.bytes();
        switch (fixtureCase.platform) {
          case "win32":
            wrongBytes.writeUInt16LE(
              fixtureCase.wrongMachine,
              fixtureCase.machineOffset,
            );
            break;
          case "darwin":
            wrongBytes.writeUInt32BE(
              fixtureCase.wrongMachine,
              fixtureCase.machineOffset,
            );
            break;
          case "linux":
            wrongBytes.writeUInt16LE(
              fixtureCase.wrongMachine,
              fixtureCase.machineOffset,
            );
            break;
          default:
            throw new Error("Unsupported fixture host");
        }
        fs.writeFileSync(wrongArchitecture, wrongBytes);
        expect(() =>
          hostNativeModule.verifyNativeTestExecutable({
            target: fixtureCase.target,
            executable: wrongArchitecture,
            platform: fixtureCase.platform,
            architecture: fixtureCase.architecture,
            cwd: ROOT,
            root: ROOT,
          }),
        ).toThrow(/does not match/);
      } finally {
        fs.rmSync(fixtureDirectory, { recursive: true, force: true });
        if (!targetDirectoryExisted) fs.rmdirSync(targetDirectory);
      }
    }
  });

  it.runIf(process.platform === "win32")(
    "passes metacharacter argv directly to a verified current-host executable",
    () => {
      const target = hostNativeModule.expectedRustTarget(
        process.platform,
        process.arch,
      ) as string;
      const targetDirectory = path.join(ROOT, "src-tauri", "target", target);
      const targetDirectoryExisted = fs.existsSync(targetDirectory);
      fs.mkdirSync(targetDirectory, { recursive: true });
      const fixtureDirectory = fs.mkdtempSync(
        path.join(targetDirectory, "fyagent-native-runner-smoke-"),
      );
      const nativeNode = path.join(fixtureDirectory, "node.exe");

      try {
        fs.copyFileSync(process.execPath, nativeNode);
        const metacharacterFilter = "filter&whoami|ignored";
        const direct = spawnSync(
          process.execPath,
          [
            "scripts/tasks/host-native.mjs",
            "native-runner",
            target,
            nativeNode,
            "-e",
            "process.stdout.write(process.argv[1])",
            metacharacterFilter,
          ],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(direct.status, output(direct)).toBe(0);
        expect(direct.stdout).toBe(metacharacterFilter);

        const outside = spawnSync(
          process.execPath,
          [
            "scripts/tasks/host-native.mjs",
            "native-runner",
            target,
            process.execPath,
          ],
          { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
        );
        expect(outside.status).not.toBe(0);
        expect(output(outside)).toContain(
          "verified current-host target directory",
        );
      } finally {
        fs.rmSync(fixtureDirectory, { recursive: true, force: true });
        if (!targetDirectoryExisted) fs.rmdirSync(targetDirectory);
      }
    },
  );

  it("rejects caller target controls before rustc, Cargo, or Tauri can start", () => {
    let childCalls = 0;
    const forbiddenChild = () => {
      childCalls += 1;
      throw new Error("child command must not start");
    };
    for (const environment of [
      { CARGO_BUILD_TARGET: foreignRustTarget() },
      { TAURI_ENV_TARGET_TRIPLE: foreignRustTarget() },
      { Rustc: "/tmp/not-the-canonical-rustc" },
      { cargo_build_rustc: "/tmp/not-the-canonical-rustc" },
      { RUSTC_WRAPPER: "/tmp/not-a-wrapper" },
      { cargo_build_rustc_workspace_wrapper: "/tmp/not-a-wrapper" },
      { RUSTDOC: "/tmp/not-the-canonical-rustdoc" },
      { cargo_build_rustdoc: "/tmp/not-the-canonical-rustdoc" },
      { Cargo_Target_Aarch64_Apple_Darwin_Runner: "/tmp/emulator" },
      { cargo_target_aarch64_apple_darwin_linker: "/tmp/linker" },
      { DYLD_INSERT_LIBRARIES: "/tmp/inject.dylib" },
      { dyld_library_path: "/tmp/inject" },
      { NODE_OPTIONS: "--require=/tmp/inject.js" },
      { RUSTFLAGS: `--target ${foreignRustTarget()}` },
      { CARGO_BUILD_RUSTFLAGS: `--target=${foreignRustTarget()}` },
      {
        cargo_target_aarch64_apple_darwin_rustflags: `-Dwarnings --target ${foreignRustTarget()}`,
      },
      {
        CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS: "-C linker=/tmp/linker",
      },
      {
        CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTDOCFLAGS:
          "-C link-arg=/tmp/inject",
      },
      {
        CARGO_ENCODED_RUSTFLAGS: `--target\u001f${foreignRustTarget()}`,
      },
      { RUSTDOCFLAGS: `--target=${foreignRustTarget()}` },
      {
        CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTDOCFLAGS: `--target ${foreignRustTarget()}`,
      },
    ]) {
      expect(() =>
        hostNativeModule.assertHostNativeGuard({
          environment,
          platform: process.platform,
          architecture: process.arch,
        }),
      ).toThrow(/canonical local tasks (?:own|reject)/);
      for (const operation of ["dev", "build", "build:binary", "build:debug"]) {
        expect(() =>
          hostNativeModule.executeTauriTask({
            operation,
            environment,
            captureCommand: forbiddenChild,
            runCommand: forbiddenChild,
          }),
        ).toThrow(/canonical local tasks (?:own|reject)/);
      }
      for (const operation of ["check", "clippy", "test"]) {
        expect(() =>
          hostNativeModule.executeCargoTask({
            operation,
            environment,
            captureCommand: forbiddenChild,
            runCommand: forbiddenChild,
          }),
        ).toThrow(/canonical local tasks (?:own|reject)/);
      }
    }
    for (const operation of ["dev", "build", "build:binary", "build:debug"]) {
      expect(() =>
        hostNativeModule.executeTauriTask({
          operation,
          forwardedArguments: ["--target", foreignRustTarget()],
          environment: {},
          captureCommand: forbiddenChild,
          runCommand: forbiddenChild,
        }),
      ).toThrow("does not accept forwarded arguments");
    }
    for (const operation of ["check", "clippy", "test"]) {
      expect(() =>
        hostNativeModule.executeCargoTask({
          operation,
          forwardedArguments: ["--target", foreignRustTarget()],
          environment: {},
          captureCommand: forbiddenChild,
          runCommand: forbiddenChild,
        }),
      ).toThrow("does not accept forwarded arguments");
    }
    expect(childCalls).toBe(0);
  });

  it.each([
    "dev",
    "build",
    "build:binary",
    "build:debug",
    "check",
    "rust:check",
    "rust:clippy",
    "rust:test",
  ])("fails mise run %s closed on a caller target environment", (task) => {
    const result = spawnSync("mise", ["run", task], {
      cwd: ROOT,
      encoding: "utf8",
      env: taskEnvironment({ CARGO_BUILD_TARGET: foreignRustTarget() }),
    });
    expect(result.status).not.toBe(0);
    expect(output(result)).toContain("CARGO_BUILD_TARGET must not be set");
    expect(output(result)).not.toMatch(
      /Compiling|Finished.*profile|beforeDevCommand|beforeBuildCommand/,
    );
  });

  it("fails aggregate check closed on compiler and runner overrides before env:check", () => {
    const overrideCases: Array<Record<string, string>> = [
      { Rustc: "/tmp/not-the-canonical-rustc" },
      { Cargo_Target_Aarch64_Apple_Darwin_Runner: "/tmp/emulator" },
    ];
    for (const overrides of overrideCases) {
      const result = spawnSync("mise", ["run", "check"], {
        cwd: ROOT,
        encoding: "utf8",
        env: taskEnvironment(overrides),
      });
      expect(result.status).not.toBe(0);
      expect(output(result)).toContain("canonical local tasks own");
      expect(output(result)).not.toContain("[env:check]");
    }
  });

  it("fails pnpm dev/build closed on forwarded args and target environment", () => {
    const target = foreignRustTarget();
    for (const [script, frontendCommand] of [
      ["dev", "beforeDevCommand"],
      ["build", "beforeBuildCommand"],
    ] as const) {
      const forwarded = spawnSync(
        resolveTaskExecutable("pnpm"),
        ["run", script, "--", "--target", target],
        { cwd: ROOT, encoding: "utf8", env: taskEnvironment({}) },
      );
      expect(forwarded.status).not.toBe(0);
      expect(output(forwarded)).toContain(
        "does not accept forwarded arguments",
      );
      expect(output(forwarded)).not.toContain(frontendCommand);
    }

    const build = spawnSync(resolveTaskExecutable("pnpm"), ["run", "build"], {
      cwd: ROOT,
      encoding: "utf8",
      env: taskEnvironment({ TAURI_ENV_TARGET_TRIPLE: target }),
    });
    expect(build.status).not.toBe(0);
    expect(output(build)).toContain("TAURI_ENV_TARGET_TRIPLE must not be set");
    expect(output(build)).not.toContain("beforeBuildCommand");
  });

  it.each(["--update", "--outputFile=vitest-results.json"])(
    "rejects the write-capable Vitest option %s before Vitest runs",
    (option) => {
      const result = mise("test:unit", "--", option);
      expect(result.status).not.toBe(0);
      expect(output(result)).toContain("Vitest options are forbidden");
      expect(output(result)).not.toContain("RUN ");
    },
  );
});
