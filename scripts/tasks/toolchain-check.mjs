#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import {
  ROOT,
  capture,
  read,
  readToml,
  run,
  usageBoolean,
  isPosixTaskHost,
} from "./lib.mjs";
import { readToolchainContract } from "../ci/verify-toolchain.mjs";
import { validateLockfile } from "./lockfile-check.mjs";

const checks = [];

function record(name, operation) {
  try {
    const detail = operation();
    checks.push({ name, ok: true, detail: detail ?? "ok" });
  } catch (error) {
    checks.push({
      name,
      ok: false,
      detail: error instanceof Error ? error.message : String(error),
    });
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
  return actual;
}

function real(value) {
  return fs.realpathSync(value);
}

function isPathInside(candidate, directory) {
  const relative = path.relative(directory, candidate);
  return (
    relative !== "" &&
    relative !== "." &&
    !relative.startsWith(`..${path.sep}`) &&
    !path.isAbsolute(relative)
  );
}

export function normalizeComparablePath(
  value,
  caseInsensitive = hostUsesCaseInsensitivePaths(),
) {
  const normalized = path
    .resolve(value)
    .replace(/\\/g, "/")
    .replace(/\/+$/, "");
  return caseInsensitive ? normalized.toLowerCase() : normalized;
}

function hostUsesCaseInsensitivePaths(platform = process.platform) {
  if (platform === "win32") return true;
  if (isPosixTaskHost(platform)) return false;
  throw new Error(`Unsupported toolchain-check host: ${platform}`);
}

function findOnPath(command) {
  let extensions;
  if (process.platform === "win32") {
    extensions = (process.env.PATHEXT ?? ".COM;.EXE;.BAT;.CMD").split(";");
  } else if (isPosixTaskHost(process.platform)) {
    extensions = [""];
  } else {
    throw new Error(`Unsupported toolchain-check host: ${process.platform}`);
  }
  for (const directory of (process.env.PATH ?? "").split(path.delimiter)) {
    if (!directory) continue;
    for (const extension of extensions) {
      const candidate = path.join(directory, `${command}${extension}`);
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile())
        return candidate;
    }
  }
  throw new Error(`${command} is not available on PATH`);
}

function verifyPython() {
  const expected = read(".python-version").trim();
  const project = readToml("pyproject.toml");
  expectEqual(
    project.project?.["requires-python"],
    ">=3.14,<3.15",
    "requires-python",
  );
  expectEqual(project.tool?.uv?.package, false, "tool.uv.package");
  expectEqual(
    project.tool?.uv?.["python-preference"],
    "only-managed",
    "tool.uv.python-preference",
  );
  const managed = capture("uv", [
    "python",
    "find",
    "--managed-python",
    "--no-python-downloads",
    "--no-project",
    expected,
  ]);
  if (!path.isAbsolute(managed))
    throw new Error(`uv returned a non-absolute Python path: ${managed}`);
  let venvPython;
  if (process.platform === "win32") {
    venvPython = path.join(process.cwd(), ".venv", "Scripts", "python.exe");
  } else if (isPosixTaskHost(process.platform)) {
    venvPython = path.join(process.cwd(), ".venv", "bin", "python");
  } else {
    throw new Error(`Unsupported toolchain-check host: ${process.platform}`);
  }
  if (!fs.existsSync(venvPython)) {
    throw new Error(".venv is missing; run mise run python:sync");
  }
  const version = capture(venvPython, [
    "-c",
    "import platform; print(platform.python_version())",
  ]);
  expectEqual(version, expected, ".venv Python");
  run("uv", ["lock", "--check", "--offline"], { capture: true });
  const locked = capture("uv", [
    "run",
    "--locked",
    "--no-sync",
    "--offline",
    "python",
    "-c",
    "import platform; print(platform.python_version())",
  ]);
  expectEqual(locked, expected, "locked uv run Python");
  return { managed, venv: real(venvPython), version };
}

function verifyTool(name, expected, versionArgs, normalize = (value) => value) {
  const actual = normalize(capture(name, versionArgs));
  expectEqual(actual, expected, `${name} version`);
  const misePath = capture("mise", ["which", name]);
  if (!path.isAbsolute(misePath) || !fs.existsSync(misePath)) {
    throw new Error(
      `mise which ${name} did not return an installed executable`,
    );
  }
  const resolved = real(misePath);
  const activePath = real(findOnPath(name));
  if (
    normalizeComparablePath(activePath) !== normalizeComparablePath(resolved)
  ) {
    throw new Error(
      `${name} PATH executable differs from mise ownership: ${activePath} != ${resolved}`,
    );
  }
  return { version: actual, path: resolved };
}

const normalizeIndex = process.argv.indexOf("--normalize-path");
if (normalizeIndex >= 0) {
  console.log(normalizeComparablePath(process.argv[normalizeIndex + 1] ?? ""));
  process.exit(0);
}

const pythonOnly = process.argv.includes("--python-only");
record("uv-managed Python and .venv", verifyPython);

if (!pythonOnly) {
  record("generated tool and lock contract", validateLockfile);
  const expected = () => readToolchainContract();
  record("Node ownership", () =>
    verifyTool("node", expected().node, ["--version"], (value) =>
      value.replace(/^v/, ""),
    ),
  );
  record("pnpm ownership", () =>
    verifyTool("pnpm", expected().pnpm, ["--version"]),
  );
  record("uv ownership", () => {
    return verifyTool(
      "uv",
      expected().uv,
      ["--version"],
      (value) => value.split(/\s+/)[1],
    );
  });
  record("Rust toolchain and components", () => {
    const expected = readToolchainContract();
    const version = capture("rustc", ["--version"]);
    if (!version.startsWith(`rustc ${expected.rust} `))
      throw new Error(version);
    const components = capture("rustup", ["component", "list", "--installed"]);
    for (const component of ["rustfmt", "clippy"]) {
      if (!new RegExp(`^${component}-`, "m").test(components)) {
        throw new Error(`Missing Rust component: ${component}`);
      }
    }
    const sysroot = capture("rustc", ["--print", "sysroot"]);
    if (
      !path.isAbsolute(sysroot) ||
      !fs.existsSync(sysroot) ||
      !sysroot.includes(expected.rust)
    )
      throw new Error(`Unexpected Rust sysroot: ${sysroot}`);
    expectEqual(
      capture("mise", ["current", "rust"]),
      expected.rust,
      "mise Rust selection",
    );
    const miseRustc = real(capture("mise", ["which", "rustc"]));
    const activeRustc = real(findOnPath("rustc"));
    const miseRustHome = real(capture("mise", ["where", "rust"]));
    for (const [label, executable] of [
      ["mise which rustc", miseRustc],
      ["PATH rustc", activeRustc],
    ]) {
      if (!isPathInside(executable, miseRustHome)) {
        throw new Error(
          `${label} is outside the mise Rust installation: ${executable} not below ${miseRustHome}`,
        );
      }
    }
    const activeToolchain = capture("rustup", ["show", "active-toolchain"]);
    if (!activeToolchain.startsWith(`${expected.rust}-`)) {
      throw new Error(`Unexpected rustup active toolchain: ${activeToolchain}`);
    }
    return {
      version,
      sysroot,
      miseRustc,
      activeRustc,
      miseRustHome,
      activeToolchain,
    };
  });
  record("mise config ownership", () => {
    const configs = JSON.parse(capture("mise", ["config", "ls", "--json"]));
    const byPath = new Map(
      configs.map((entry) => [normalizeComparablePath(entry.path), entry]),
    );
    const rootConfig = byPath.get(
      normalizeComparablePath(path.join(ROOT, "mise.toml")),
    );
    expectEqual(
      JSON.stringify(rootConfig?.tools),
      JSON.stringify(["uv"]),
      "mise.toml tools",
    );
    for (const relative of [
      ".node-version",
      "package.json",
      "rust-toolchain.toml",
    ]) {
      const expectedPath = normalizeComparablePath(path.join(ROOT, relative));
      if (!byPath.has(expectedPath)) {
        throw new Error(`mise did not load idiomatic source ${relative}`);
      }
    }
    return configs;
  });
  record("mise task metadata", () => {
    run("mise", ["tasks", "validate", "--errors-only"], { capture: true });
    const tasks = JSON.parse(
      capture("mise", ["tasks", "ls", "--local", "--json"]),
    );
    const names = new Set(tasks.map((task) => task.name));
    if (names.size === 0) throw new Error("mise did not load any local tasks");
    return `${names.size} tasks`;
  });
  record("standard version facts", expected);
}

const report = {
  ok: checks.every((check) => check.ok),
  platform: `${process.platform}-${process.arch}`,
  hostname: os.hostname(),
  checks,
};

if (usageBoolean("json") || process.argv.includes("--json")) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log(`FyAgent environment (${report.platform}):`);
  for (const check of checks) {
    console.log(`  ${check.ok ? "PASS" : "FAIL"} ${check.name}`);
    if (!check.ok) console.log(`       ${check.detail}`);
  }
}
if (!report.ok) process.exitCode = 1;
