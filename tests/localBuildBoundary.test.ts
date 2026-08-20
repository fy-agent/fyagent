import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const read = (relative: string) =>
  fs.readFileSync(path.join(ROOT, relative), "utf8").replace(/\r\n/g, "\n");

type CommandPlan = {
  command: string;
  args: string[];
  target: string;
  environment: Record<string, string>;
};
type HostNativeModule = {
  HOST_RUST_TARGETS: Readonly<Record<string, string>>;
  expectedRustTarget(platform: string, architecture: string): string;
  parseRustcHost(verboseVersion: string): string;
  assertCurrentRustHost(input: {
    platform: string;
    architecture: string;
    rustcVerboseVersion: string;
  }): string;
  containsRustTargetOverride(value: string, encoded?: boolean): boolean;
  assertNoCallerTargetOverride(environment: Record<string, string>): void;
  buildNativeRunnerConfig(input: {
    target: string;
    platform: string;
    nodeExecutable: string;
    runnerScript: string;
  }): string;
  planTauriTask(input: {
    operation: string;
    forwardedArguments?: string[];
    environment: Record<string, string>;
    platform: string;
    architecture: string;
    rustcVerboseVersion: string;
    rustdocVerboseVersion: string;
    rustcExecutable: string;
    rustdocExecutable: string;
  }): CommandPlan;
  planCargoTask(input: {
    operation: string;
    filters?: string[];
    forwardedArguments?: string[];
    environment: Record<string, string>;
    platform: string;
    architecture: string;
    rustcVerboseVersion: string;
    rustdocVerboseVersion: string;
    rustcExecutable: string;
    rustdocExecutable: string;
    nativeRunnerConfig: string;
  }): CommandPlan;
};

let hostNative: HostNativeModule;

beforeAll(async () => {
  hostNative = (await import(
    /* @vite-ignore */ pathToFileURL(
      path.join(ROOT, "scripts", "tasks", "host-native.mjs"),
    ).href
  )) as HostNativeModule;
});

const RETIRED_TASKS = [
  "macos:preflight",
  "build:cross-windows:x64",
  "build:cross-windows:arm64",
  "build:cross-windows",
  "build:cross-macos:universal",
];
const RETIRED_PATHS = ["scripts/macos-cross", "scripts/windows-cross"];
const LOCAL_CROSS_EXECUTION_MARKERS = [
  "--target",
  "CARGO_BUILD_TARGET",
  "TAURI_ENV_TARGET_TRIPLE",
  "cargo-xwin",
  "cargo-zigbuild",
  "cross build",
  "osxcross",
  "qemu",
  "wine",
  "scripts/macos-cross",
  "scripts/windows-cross",
  "src-tauri/target/app",
  "universal-apple-darwin",
  "pc-windows-msvc",
  "rustup target add",
];
const CURRENT_DOCUMENTS = [
  "README.md",
  "README_EN.md",
  "README_JA.md",
  "CONTRIBUTING.md",
  "docs/fyagent/development/tooling/mise.md",
  "docs/fyagent/development/validation.md",
  "docs/fyagent/development/windows/installer.md",
  "docs/fyagent/development/windows/codex-desktop.md",
];

function executableRepositoryFiles(): string[] {
  const roots = [
    "mise.toml",
    "mise.lock",
    "scripts",
    ".github/workflows",
    ".mise",
  ];
  const files: string[] = [];
  for (const relativeRoot of roots) {
    const absoluteRoot = path.join(ROOT, relativeRoot);
    if (!fs.existsSync(absoluteRoot)) continue;
    if (fs.statSync(absoluteRoot).isFile()) {
      files.push(absoluteRoot);
      continue;
    }
    const visit = (directory: string) => {
      for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        const absolute = path.join(directory, entry.name);
        if (entry.isDirectory()) visit(absolute);
        else if (entry.isFile()) files.push(absolute);
      }
    };
    visit(absoluteRoot);
  }
  return files;
}

function localTaskConfiguration(): string {
  const packageJson = JSON.parse(read("package.json")) as {
    scripts: Record<string, string>;
  };
  return [
    read("mise.toml"),
    JSON.stringify(packageJson.scripts),
    ...fs
      .readdirSync(path.join(ROOT, ".mise/tasks"))
      .filter((name) => name.endsWith(".toml"))
      .map((name) => read(path.posix.join(".mise/tasks", name))),
  ].join("\n");
}

describe("local build boundary", () => {
  it("removes every local cross-OS build entrypoint and dedicated contract", () => {
    for (const retiredPath of RETIRED_PATHS) {
      expect(fs.existsSync(path.join(ROOT, retiredPath))).toBe(false);
    }
    expect(
      fs.existsSync(path.join(ROOT, "tests/macosCrossWorkflow.test.ts")),
    ).toBe(false);
    const taskSources = localTaskConfiguration();
    for (const task of RETIRED_TASKS) {
      expect(taskSources).not.toContain(`[tasks."${task}"]`);
      expect(taskSources).not.toContain(`["${task}"]`);
    }
    for (const retiredPath of RETIRED_PATHS) {
      expect(taskSources).not.toContain(retiredPath);
    }
    expect(taskSources).not.toContain("llvm-tools");
    expect(taskSources).not.toMatch(/^targets\s*=/m);

    const lock = read("mise.lock");
    expect(lock).not.toContain("llvm-tools");
    expect(lock).not.toMatch(/^targets\s*=/m);
  });

  it("keeps standard local development, builds, and tests current-host-only", () => {
    const packageJson = JSON.parse(read("package.json")) as {
      scripts: Record<string, string>;
    };
    expect(packageJson.scripts.dev).toBe(
      "node scripts/tasks/host-native.mjs dev",
    );
    expect(packageJson.scripts.build).toBe(
      "node scripts/tasks/host-native.mjs build",
    );
    expect(packageJson.scripts.tauri).toBe("tauri");

    const nativeTasks = read(".mise/tasks/core.toml");
    for (const task of ["dev", "build", "build:binary", "build:debug"]) {
      expect(nativeTasks).toContain(
        task.includes(":") ? `["${task}"]` : `[${task}]`,
      );
    }
    expect(nativeTasks).not.toContain("--target");
    expect(nativeTasks).toContain(
      'run = "node scripts/tasks/host-native.mjs build:binary"',
    );
    expect(nativeTasks).toContain(
      'run = "node scripts/tasks/host-native.mjs build:debug"',
    );
    const rustTasks = read(".mise/tasks/rust.toml");
    for (const operation of ["check", "clippy", "test"]) {
      expect(rustTasks).toContain(
        `run = "node scripts/tasks/rust.mjs ${operation}"`,
      );
    }

    const localEntryPoints = localTaskConfiguration();
    for (const marker of LOCAL_CROSS_EXECUTION_MARKERS) {
      expect(localEntryPoints, marker).not.toContain(marker);
    }

    for (const document of CURRENT_DOCUMENTS.slice(0, 4)) {
      const content = read(document);
      expect(content).toContain("mise run dev");
      expect(content).toContain("mise run build");
      expect(content).not.toContain("dist-bundle/");
    }
  });

  it("maps every development-host process pair and verifies rustc identity", () => {
    const cases = [
      ["darwin", "x64", "x86_64-apple-darwin"],
      ["darwin", "arm64", "aarch64-apple-darwin"],
      ["win32", "x64", "x86_64-pc-windows-msvc"],
      ["win32", "arm64", "aarch64-pc-windows-msvc"],
      ["linux", "x64", "x86_64-unknown-linux-gnu"],
      ["linux", "arm64", "aarch64-unknown-linux-gnu"],
    ] as const;
    expect(Object.keys(hostNative.HOST_RUST_TARGETS)).toHaveLength(6);
    for (const [platform, architecture, target] of cases) {
      expect(hostNative.expectedRustTarget(platform, architecture)).toBe(
        target,
      );
      expect(
        hostNative.assertCurrentRustHost({
          platform,
          architecture,
          rustcVerboseVersion: `rustc 1.97.1\nhost: ${target}\nrelease: 1.97.1`,
        }),
      ).toBe(target);
    }
    expect(() => hostNative.expectedRustTarget("freebsd", "x64")).toThrow(
      "Unsupported local host OS/architecture",
    );
    expect(() =>
      hostNative.assertCurrentRustHost({
        platform: "darwin",
        architecture: "x64",
        rustcVerboseVersion: "host: aarch64-apple-darwin",
      }),
    ).toThrow("does not match current host");
    expect(() => hostNative.parseRustcHost("rustc 1.97.1")).toThrow(
      "exactly one host target",
    );
  });

  it("rejects caller-owned target environment and Rust flag tokens", () => {
    const targetOverrideEnvironments: Array<Record<string, string>> = [
      { CARGO_BUILD_TARGET: "" },
      { TAURI_ENV_TARGET_TRIPLE: "aarch64-apple-darwin" },
      { Rustc: "/tmp/not-the-canonical-rustc" },
      { cargo_build_rustc: "/tmp/not-the-canonical-rustc" },
      { RUSTC_WRAPPER: "/tmp/not-a-wrapper" },
      { cargo_build_rustc_workspace_wrapper: "/tmp/not-a-wrapper" },
      { RUSTDOC: "/tmp/not-the-canonical-rustdoc" },
      { cargo_build_rustdoc: "/tmp/not-the-canonical-rustdoc" },
      { Cargo_Target_Aarch64_Apple_Darwin_Runner: "/tmp/emulator" },
      { cargo_target_aarch64_apple_darwin_linker: "/tmp/linker" },
      { DYLD_INSERT_LIBRARIES: "/tmp/inject.dylib" },
      { node_options: "--require=/tmp/inject.js" },
      { RUSTFLAGS: "-Dwarnings --target aarch64-apple-darwin" },
      {
        cargo_target_aarch64_apple_darwin_rustflags:
          "-Dwarnings --target aarch64-apple-darwin",
      },
      {
        CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS: "-C linker=/tmp/linker",
      },
      {
        CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTDOCFLAGS:
          "-C link-arg=/tmp/inject",
      },
      {
        CARGO_ENCODED_RUSTFLAGS:
          "-Dwarnings\u001f--target=aarch64-apple-darwin",
      },
      {
        CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTDOCFLAGS:
          "--target=aarch64-apple-darwin",
      },
    ];
    for (const environment of targetOverrideEnvironments) {
      expect(() =>
        hostNative.assertNoCallerTargetOverride(environment),
      ).toThrow(/canonical local tasks (?:own|reject)/);
    }
    expect(hostNative.containsRustTargetOverride("-Dwarnings --cfg test")).toBe(
      false,
    );
    expect(() =>
      hostNative.assertNoCallerTargetOverride({
        RUSTFLAGS: "-Dwarnings --cfg test",
      }),
    ).not.toThrow();
  });

  it("plans fixed Tauri and Cargo argv with the verified current host target", () => {
    const base = {
      environment: {},
      platform: "darwin",
      architecture: "x64",
      rustcVerboseVersion:
        "rustc 1.97.1\ncommit-hash: verified-toolchain\nhost: x86_64-apple-darwin\nrelease: 1.97.1",
      rustdocVerboseVersion:
        "rustdoc 1.97.1\ncommit-hash: verified-toolchain\nhost: x86_64-apple-darwin\nrelease: 1.97.1",
      rustcExecutable: "/toolchain/bin/rustc",
      rustdocExecutable: "/toolchain/bin/rustdoc",
      nativeRunnerConfig:
        'target.x86_64-apple-darwin.runner=["/usr/bin/node","/repo/scripts/tasks/host-native.mjs","native-runner","x86_64-apple-darwin"]',
    };
    const dev = hostNative.planTauriTask({ ...base, operation: "dev" });
    expect(dev).toMatchObject({
      command: "pnpm",
      args: ["tauri", "dev", "--target", "x86_64-apple-darwin"],
      target: "x86_64-apple-darwin",
    });
    expect(dev.environment).toMatchObject({
      RUSTC: "/toolchain/bin/rustc",
      CARGO_BUILD_RUSTC: "/toolchain/bin/rustc",
      RUSTDOC: "/toolchain/bin/rustdoc",
      CARGO_BUILD_RUSTDOC: "/toolchain/bin/rustdoc",
      RUSTC_WRAPPER: "",
      RUSTC_WORKSPACE_WRAPPER: "",
      RUSTFLAGS: "",
      CARGO_ENCODED_RUSTFLAGS: "",
      RUSTDOCFLAGS: "",
      CARGO_ENCODED_RUSTDOCFLAGS: "",
    });
    expect(
      hostNative.planTauriTask({ ...base, operation: "build:binary" }).args,
    ).toEqual([
      "tauri",
      "build",
      "--target",
      "x86_64-apple-darwin",
      "--no-bundle",
    ]);
    expect(
      hostNative.planTauriTask({ ...base, operation: "build:debug" }).args,
    ).toEqual(["tauri", "build", "--target", "x86_64-apple-darwin", "--debug"]);
    expect(
      hostNative.planCargoTask({ ...base, operation: "check" }).args,
    ).toEqual([
      "--config",
      base.nativeRunnerConfig,
      "check",
      "--target",
      "x86_64-apple-darwin",
      "--workspace",
      "--locked",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--all-targets",
    ]);
    expect(
      hostNative.planCargoTask({
        ...base,
        operation: "test",
        filters: ["settings"],
      }).args,
    ).toEqual([
      "--config",
      base.nativeRunnerConfig,
      "test",
      "--target",
      "x86_64-apple-darwin",
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
    expect(
      hostNative
        .planCargoTask({
          ...base,
          operation: "test",
          filters: ["settings&whoami|ignored"],
        })
        .args.slice(-2),
    ).toEqual(["--", "settings&whoami|ignored"]);
    expect(() =>
      hostNative.planTauriTask({
        ...base,
        operation: "build",
        forwardedArguments: ["--target", "aarch64-pc-windows-msvc"],
      }),
    ).toThrow("does not accept forwarded arguments");
  });

  it("encodes the no-shell native runner as fixed argv even when Windows paths contain spaces", () => {
    const nodeExecutable = "C:\\Program Files\\Node JS\\node.exe";
    const runnerScript =
      "C:\\Work Space\\FyAgent\\scripts\\tasks\\host-native.mjs";
    const runner = hostNative.buildNativeRunnerConfig({
      target: "x86_64-pc-windows-msvc",
      platform: "win32",
      nodeExecutable,
      runnerScript,
    });
    expect(runner).toBe(
      `target.x86_64-pc-windows-msvc.runner=${JSON.stringify([
        nodeExecutable,
        runnerScript,
        "native-runner",
        "x86_64-pc-windows-msvc",
      ])}`,
    );
    expect(runner).not.toMatch(
      /cmd(?:\.exe)?|powershell|pwsh|(?:^|\s)-c(?:\s|$)/i,
    );
  });

  it("prevents repository tasks and scripts from changing mise trust", () => {
    const miseTrustMutation =
      /(?:\bmise(?:\.exe)?\b|\/[^\s"'`]*\/mise\b|[A-Za-z]:\\[^\s"'`]*\\mise(?:\.exe)?\b|\$\{?[A-Za-z_][A-Za-z0-9_]*MISE[A-Za-z0-9_]*\}?)[^\r\n]*\b(?:trust|untrust)\b/i;
    for (const file of executableRepositoryFiles()) {
      const relative = path.relative(ROOT, file);
      const content = read(relative);
      expect(content, relative).not.toMatch(miseTrustMutation);
    }
  });

  it("keeps current documents free of retired cross-build interfaces", () => {
    for (const document of CURRENT_DOCUMENTS) {
      const content = read(document);
      for (const task of RETIRED_TASKS) {
        expect(content, document).not.toContain(task);
      }
      for (const retiredPath of RETIRED_PATHS) {
        expect(content, document).not.toContain(retiredPath);
      }
    }
  });

  it("retains native release targets for all three platform groups", () => {
    const release = read(".github/workflows/release.yml");
    for (const contract of [
      "runner: windows-2025",
      "target_group: windows-x64",
      "runner: windows-11-arm",
      "target_group: windows-arm64",
      "runs-on: macos-15",
      "TARGET_GROUP: macos-universal",
    ]) {
      expect(release).toContain(contract);
    }

    expect(release).toContain("aarch64-pc-windows-msvc");
    expect(release).toContain("pnpm tauri build --no-bundle");
    expect(release).toContain(
      "pnpm tauri build --target universal-apple-darwin",
    );
    expect(release).toContain("FYAGENT_WINDOWS_MANIFEST: release");
  });
});
