#!/usr/bin/env node

import process from "node:process";
import {
  assertNoCallerTargetOverride,
  assertNoCargoToolchainConfig,
} from "./host-native.mjs";
import { ROOT, fail, isMain, run, usageBoolean } from "./lib.mjs";

export const WINDOWS_MSVC_CROSS_TARGET = "x86_64-pc-windows-msvc";
export const CARGO_XWIN_VERSION = "0.23.1";
export const WINDOWS_MSVC_CROSS_HOST_TARGETS = Object.freeze({
  "darwin-x64": WINDOWS_MSVC_CROSS_TARGET,
  "darwin-arm64": WINDOWS_MSVC_CROSS_TARGET,
});

const WINDOWS_MSVC_CROSS_OVERRIDES = Object.freeze(
  new Set([
    "AR",
    "ARFLAGS",
    "BINDGEN_EXTRA_CLANG_ARGS",
    "CC",
    "CFLAGS",
    "CMAKE",
    "CMAKE_GENERATOR",
    "CMAKE_PREFIX_PATH",
    "CMAKE_TOOLCHAIN_FILE",
    "CPPFLAGS",
    "CXX",
    "CXXFLAGS",
    "INCLUDE",
    "LDFLAGS",
    "LIB",
    "LINK",
    "NM",
    "PERL_EXECUTABLE",
    "RANLIB",
    "RUSTDOCFLAGS",
    "RUSTFLAGS",
    "CARGO_BUILD_RUSTDOCFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_ENCODED_RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
  ]),
);

const WINDOWS_MSVC_CROSS_OVERRIDE_PREFIXES = Object.freeze([
  "AR_",
  "AWS_LC_",
  "CARGO_XWIN_",
  "CC_",
  "CFLAGS_",
  "CMAKE_",
  "CPPFLAGS_",
  "CXX_",
  "CXXFLAGS_",
  "LDFLAGS_",
  "RANLIB_",
  "RING_",
  "XWIN_",
]);

const REQUIREMENTS = Object.freeze([
  Object.freeze({
    id: "cargo-xwin",
    command: "cargo",
    args: Object.freeze(["xwin", "--version"]),
    hint: `Install the reviewed cargo-xwin ${CARGO_XWIN_VERSION} release with: cargo install --locked cargo-xwin --version ${CARGO_XWIN_VERSION}`,
  }),
  Object.freeze({
    id: "clippy",
    command: "cargo",
    args: Object.freeze(["clippy", "--version"]),
    hint: "Install the Clippy component for the repository Rust toolchain with: rustup component add clippy",
  }),
  Object.freeze({
    id: "rust-target",
    command: "rustup",
    args: Object.freeze(["target", "list", "--installed"]),
    hint: `Install the reviewed Rust target with: rustup target add ${WINDOWS_MSVC_CROSS_TARGET}`,
  }),
  Object.freeze({
    id: "clang-cl",
    command: "clang-cl",
    args: Object.freeze(["--version"]),
    hint: "Install a full LLVM toolchain and expose its bin directory on PATH (cargo-xwin documents brew install llvm on macOS).",
  }),
  Object.freeze({
    id: "lld-link",
    command: "lld-link",
    args: Object.freeze(["--version"]),
    hint: "Expose lld-link from the same full LLVM installation on PATH.",
  }),
  Object.freeze({
    id: "llvm-lib",
    command: "llvm-lib",
    args: Object.freeze(["--version"]),
    hint: "Expose llvm-lib from the same full LLVM installation on PATH.",
  }),
  Object.freeze({
    id: "cmake",
    command: "cmake",
    args: Object.freeze(["--version"]),
    hint: "Install CMake for native C/C++ dependency build scripts.",
  }),
  Object.freeze({
    id: "ninja",
    command: "ninja",
    args: Object.freeze(["--version"]),
    hint: "Install Ninja; cargo-xwin requires it for its CMake integration.",
  }),
]);

function boundedDetail(result) {
  const value = `${result.stderr ?? ""}\n${result.stdout ?? ""}`
    .trim()
    .split(/\r?\n/u)[0];
  if (!value) return undefined;
  return value.length > 240 ? `${value.slice(0, 237)}...` : value;
}

function probe(runCommand, requirement) {
  try {
    const result = runCommand(requirement.command, [...requirement.args], {
      capture: true,
      allowFailure: true,
    });
    return {
      status: result.status,
      stdout: result.stdout ?? "",
      stderr: result.stderr ?? "",
    };
  } catch (error) {
    return {
      status: 1,
      stdout: "",
      stderr: error instanceof Error ? error.message : String(error),
    };
  }
}

export function parseCargoXwinVersion(output) {
  const lines = String(output)
    .replace(/\r\n/gu, "\n")
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  const match =
    lines.length === 1
      ? /^cargo-xwin(?:-xwin)?\s+(\d+\.\d+\.\d+)$/u.exec(lines[0])
      : null;
  if (!match) {
    throw new Error(
      "cargo xwin --version must report exactly one stable version",
    );
  }
  return match[1];
}

export function assertNoWindowsMsvcCrossOverrides(environment) {
  assertNoCallerTargetOverride(environment);
  assertNoCargoToolchainConfig({ root: ROOT, environment });
  for (const name of Object.keys(environment)) {
    const normalized = name.toUpperCase();
    if (
      WINDOWS_MSVC_CROSS_OVERRIDE_PREFIXES.some((prefix) =>
        normalized.startsWith(prefix),
      ) ||
      WINDOWS_MSVC_CROSS_OVERRIDES.has(normalized)
    ) {
      throw new Error(
        `${name} must not be set; the Windows MSVC cross diagnostic owns its compiler and SDK inputs`,
      );
    }
  }
}

export function expectedWindowsMsvcCrossTarget(platform, architecture) {
  const target = WINDOWS_MSVC_CROSS_HOST_TARGETS[`${platform}-${architecture}`];
  if (!target) {
    throw new Error(
      `Windows MSVC cross diagnostics require macOS x64/arm64, received ${platform}/${architecture}`,
    );
  }
  return target;
}

export function inspectWindowsMsvcCross({
  platform = process.platform,
  architecture = process.arch,
  environment = process.env,
  runCommand = run,
} = {}) {
  try {
    expectedWindowsMsvcCrossTarget(platform, architecture);
  } catch (error) {
    return {
      ok: false,
      platform,
      target: WINDOWS_MSVC_CROSS_TARGET,
      checks: [
        {
          id: "supported-host",
          name: "macOS diagnostic host",
          ok: false,
          hint: "This optional diagnostic is supported only on macOS; use the native Windows CI/HIL gate for Windows acceptance.",
          detail: error instanceof Error ? error.message : String(error),
        },
      ],
    };
  }

  try {
    assertNoWindowsMsvcCrossOverrides(environment);
  } catch (error) {
    return {
      ok: false,
      platform,
      target: WINDOWS_MSVC_CROSS_TARGET,
      checks: [
        {
          id: "caller-environment",
          name: "caller toolchain environment",
          ok: false,
          hint: error instanceof Error ? error.message : String(error),
        },
      ],
    };
  }

  const checks = [];
  for (const requirement of REQUIREMENTS) {
    const result = probe(runCommand, requirement);
    let ok = result.status === 0;
    let hint = ok ? undefined : requirement.hint;
    let detail = ok ? undefined : boundedDetail(result);

    if (requirement.id === "cargo-xwin" && ok) {
      try {
        const actual = parseCargoXwinVersion(
          `${result.stdout}\n${result.stderr}`,
        );
        ok = actual === CARGO_XWIN_VERSION;
        if (!ok) {
          hint = `cargo-xwin must be exactly ${CARGO_XWIN_VERSION}; found ${actual}.`;
          detail = undefined;
        }
      } catch (error) {
        ok = false;
        hint = error instanceof Error ? error.message : String(error);
        detail = boundedDetail(result);
      }
    }

    if (requirement.id === "rust-target" && ok) {
      const installed = new Set(
        result.stdout
          .split(/\r?\n/u)
          .map((value) => value.trim())
          .filter(Boolean),
      );
      ok = installed.has(WINDOWS_MSVC_CROSS_TARGET);
      if (!ok) {
        hint = requirement.hint;
        detail = undefined;
      }
    }

    checks.push({
      id: requirement.id,
      name: `${requirement.command} ${requirement.args.join(" ")}`,
      ok,
      hint,
      detail,
    });
  }

  return {
    ok: checks.every((check) => check.ok),
    platform,
    target: WINDOWS_MSVC_CROSS_TARGET,
    checks,
  };
}

export function planWindowsMsvcCrossClippy({
  platform = process.platform,
  architecture = process.arch,
  environment = process.env,
  forwardedArguments = [],
} = {}) {
  expectedWindowsMsvcCrossTarget(platform, architecture);
  if (forwardedArguments.length > 0) {
    throw new Error(
      "Windows MSVC cross Clippy does not accept forwarded arguments",
    );
  }
  assertNoWindowsMsvcCrossOverrides(environment);
  return {
    command: "cargo",
    args: [
      "xwin",
      "clippy",
      "--cross-compiler",
      "clang-cl",
      "--xwin-version",
      "17",
      "--target",
      WINDOWS_MSVC_CROSS_TARGET,
      "--workspace",
      "--all-targets",
      "--locked",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--",
      "-D",
      "warnings",
    ],
  };
}

export function printWindowsMsvcCrossReport(report, json = false) {
  if (json) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }
  console.log(`Windows MSVC cross prerequisites (${report.platform}):`);
  for (const check of report.checks) {
    console.log(`  ${check.ok ? "PASS" : "FAIL"} ${check.name}`);
    if (check.detail) console.log(`       ${check.detail}`);
    if (check.hint) console.log(`       ${check.hint}`);
  }
  console.log(
    "  NOTE This result is a cross-compilation diagnostic, not Windows native runtime, installer, signing, or HIL evidence.",
  );
}

function main() {
  const mode = process.argv[2];
  if (!new Set(["advisory", "check", "clippy"]).has(mode)) {
    throw new Error(`Unknown Windows MSVC cross task mode: ${mode ?? ""}`);
  }
  if (mode === "advisory") {
    try {
      expectedWindowsMsvcCrossTarget(process.platform, process.arch);
    } catch {
      console.log(
        `Windows MSVC cross prerequisites (${process.platform}): SKIP optional macOS-only diagnostic`,
      );
      return;
    }
  }
  const report = inspectWindowsMsvcCross();
  const json = usageBoolean("json") || process.argv.includes("--json");
  printWindowsMsvcCrossReport(report, json);
  if (!report.ok) {
    if (mode === "advisory") {
      console.log(
        "  ADVISORY Optional Windows MSVC cross-Clippy prerequisites are incomplete; bootstrap remains valid. Run `mise run system:check:windows-msvc-cross` for the strict preflight.",
      );
      return;
    }
    process.exitCode = 1;
    return;
  }
  if (mode === "check" || mode === "advisory") return;

  console.log(
    "cargo-xwin may download and cache the Microsoft CRT/Windows SDK; using it accepts the Microsoft license referenced by cargo-xwin.",
  );
  const plan = planWindowsMsvcCrossClippy();
  run(plan.command, plan.args);
}

if (isMain(import.meta.url)) {
  try {
    main();
  } catch (error) {
    fail(error);
  }
}
