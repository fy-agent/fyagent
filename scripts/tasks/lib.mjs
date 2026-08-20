#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parse as parseToml } from "smol-toml";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));

export const ROOT = path.resolve(scriptDirectory, "..", "..");
export const PRODUCT_PLATFORMS = Object.freeze([
  "macos-x64",
  "macos-arm64",
  "windows-x64",
  "windows-arm64",
]);
export const DEVELOPMENT_HOSTS = Object.freeze([
  ...PRODUCT_PLATFORMS,
  "linux-x64",
  "linux-arm64",
]);
export const SUPPORTED_PLATFORMS = DEVELOPMENT_HOSTS;
export const STABLE_SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export function isPosixTaskHost(platform) {
  return platform === "darwin" || platform === "linux";
}

export function resolveTaskExecutable(command, platform = process.platform) {
  if (platform === "win32") {
    return command === "pnpm" ? "pnpm.exe" : command;
  }
  if (isPosixTaskHost(platform)) {
    return command;
  }
  throw new Error(`Unsupported task host: ${platform}`);
}

export function run(command, args = [], options = {}) {
  const result = spawnSync(resolveTaskExecutable(command), args, {
    cwd: ROOT,
    env: { ...process.env, ...(options.env ?? {}) },
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0 && !options.allowFailure) {
    const detail = options.capture
      ? `\n${(result.stderr || result.stdout || "").trim()}`
      : "";
    throw new Error(
      `${command} ${args.join(" ")} exited with ${result.status}${detail}`,
    );
  }
  return {
    status: result.status ?? 1,
    stdout: (result.stdout ?? "").trim(),
    stderr: (result.stderr ?? "").trim(),
  };
}

export function capture(command, args = [], options = {}) {
  return run(command, args, { ...options, capture: true }).stdout;
}

export function read(relativePath) {
  return fs
    .readFileSync(path.join(ROOT, relativePath), "utf8")
    .replace(/\r\n/g, "\n");
}

export function readToml(relativePath) {
  return parseToml(read(relativePath));
}

export function readJson(relativePath) {
  return JSON.parse(read(relativePath));
}

export function usageBoolean(name) {
  return /^(1|true|yes)$/i.test(process.env[`usage_${name}`] ?? "");
}

export function usageValue(name) {
  const value = process.env[`usage_${name}`];
  return value === undefined || value === "" ? undefined : value;
}

// mise serializes variadic usage values as a shell-escaped string. Parse only
// quoting and escaping here; commands are always spawned with an argv array.
export function usageList(name) {
  const input = usageValue(name);
  if (!input) return [];
  const output = [];
  let current = "";
  let quote = null;
  let escaped = false;
  let started = false;
  for (const character of input) {
    if (escaped) {
      current += character;
      escaped = false;
      started = true;
      continue;
    }
    if (character === "\\" && quote !== "'") {
      escaped = true;
      started = true;
      continue;
    }
    if (quote) {
      if (character === quote) quote = null;
      else current += character;
      started = true;
      continue;
    }
    if (character === "'" || character === '"') {
      quote = character;
      started = true;
      continue;
    }
    if (/\s/.test(character)) {
      if (started) {
        output.push(current);
        current = "";
        started = false;
      }
      continue;
    }
    current += character;
    started = true;
  }
  if (escaped || quote) throw new Error(`Malformed usage_${name} value`);
  if (started) output.push(current);
  return output;
}

export function assertStableSemver(value, label = "version") {
  if (!value || !STABLE_SEMVER.test(value)) {
    throw new Error(`${label} must be an exact stable X.Y.Z version`);
  }
  return value;
}

export function assertSimplePackageNames(values, label = "package") {
  if (values.length === 0) throw new Error(`At least one ${label} is required`);
  for (const value of values) {
    if (!/^(?:@[-a-z0-9_.]+\/)?[-a-z0-9_.]+(?:@[^\s]+)?$/i.test(value)) {
      throw new Error(`Invalid ${label}: ${value}`);
    }
  }
  return values;
}

export function repositoryPath(relativePath) {
  const absolute = path.resolve(ROOT, relativePath);
  const relative = path.relative(ROOT, absolute);
  if (
    relative === "" ||
    relative === "." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error(`Path must be a child of the repository: ${relativePath}`);
  }
  return absolute;
}

function writeTemporaryFile(absolute, content, mode, temporaryPaths) {
  const temporaryPath = path.join(
    path.dirname(absolute),
    `.${path.basename(absolute)}.fyagent-task-${process.pid}-${randomUUID()}.tmp`,
  );
  const descriptor = fs.openSync(temporaryPath, "wx", mode ?? 0o666);
  temporaryPaths.add(temporaryPath);

  let writeError;
  try {
    fs.writeFileSync(descriptor, content);
  } catch (error) {
    writeError = error;
  }

  try {
    fs.closeSync(descriptor);
  } catch (closeError) {
    if (writeError !== undefined) {
      throw new AggregateError(
        [writeError, closeError],
        "Writing and closing an atomic temporary file both failed",
        { cause: writeError },
      );
    }
    throw closeError;
  }
  if (writeError !== undefined) throw writeError;

  // rename inherits the temporary file's permissions, not the destination's.
  if (mode !== null) fs.chmodSync(temporaryPath, mode);
  return temporaryPath;
}

function cleanupTemporaryFiles(temporaryPaths, recoveryErrors) {
  for (const temporaryPath of temporaryPaths) {
    try {
      fs.rmSync(temporaryPath, { force: true });
    } catch (error) {
      recoveryErrors.push(error);
    }
  }
  temporaryPaths.clear();
}

export function writeFilesAtomically(changes) {
  const originals = new Map();
  const temporaryPaths = new Set();
  const staged = [];
  const replaced = [];
  try {
    for (const [relativePath, content] of changes) {
      const absolute = repositoryPath(relativePath);
      const exists = fs.existsSync(absolute);
      const original = exists
        ? {
            content: fs.readFileSync(absolute),
            mode: fs.statSync(absolute).mode & 0o7777,
          }
        : { content: null, mode: null };
      originals.set(absolute, original);
      staged.push([
        writeTemporaryFile(absolute, content, original.mode, temporaryPaths),
        absolute,
      ]);
    }
    for (const [temporaryPath, absolute] of staged) {
      fs.renameSync(temporaryPath, absolute);
      temporaryPaths.delete(temporaryPath);
      replaced.push(absolute);
    }
  } catch (primaryError) {
    const recoveryErrors = [];
    cleanupTemporaryFiles(temporaryPaths, recoveryErrors);

    for (const absolute of replaced.reverse()) {
      const original = originals.get(absolute);
      try {
        if (original.content === null) {
          fs.rmSync(absolute, { force: true });
        } else {
          const temporaryPath = writeTemporaryFile(
            absolute,
            original.content,
            original.mode,
            temporaryPaths,
          );
          fs.renameSync(temporaryPath, absolute);
          temporaryPaths.delete(temporaryPath);
        }
      } catch (error) {
        recoveryErrors.push(error);
      }
    }
    cleanupTemporaryFiles(temporaryPaths, recoveryErrors);

    if (recoveryErrors.length > 0) {
      const detail =
        primaryError instanceof Error
          ? primaryError.message
          : String(primaryError);
      throw new AggregateError(
        [primaryError, ...recoveryErrors],
        `Atomic file write failed: ${detail}; ${recoveryErrors.length} recovery operation(s) also failed`,
        { cause: primaryError },
      );
    }
    throw primaryError;
  }
}

export function isMain(importMetaUrl) {
  if (!process.argv[1]) return false;
  return pathToFileURL(path.resolve(process.argv[1])).href === importMetaUrl;
}

export function printPlan(title, command, args) {
  console.log(
    JSON.stringify(
      { status: "preview", title, command: [command, ...args] },
      null,
      2,
    ),
  );
}

export function fail(error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
