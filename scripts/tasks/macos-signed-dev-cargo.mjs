#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

function fail(message) {
  throw new Error(`macos-signed-dev-cargo: ${message}`);
}

function existingFile(file, label, { allowSymlink = false } = {}) {
  if (!file) fail(`${label} is missing`);
  const absolute = path.resolve(file);
  let link;
  try {
    link = fs.lstatSync(absolute);
  } catch {
    fail(`${label} does not exist: ${absolute}`);
  }
  if (link.isSymbolicLink() && !allowSymlink) {
    fail(`${label} must not be a symlink: ${absolute}`);
  }
  let stat;
  try {
    stat = link.isSymbolicLink() ? fs.statSync(absolute) : link;
  } catch {
    fail(`${label} symlink target does not exist: ${absolute}`);
  }
  if (!stat.isFile() || stat.size <= 0) {
    fail(
      `${label} must be a non-empty regular${allowSymlink ? "" : " non-symlink"} file: ${absolute}`,
    );
  }
  return absolute;
}

function tomlString(value) {
  return JSON.stringify(value);
}

function targetRunnerConfig(target, node, appRunner) {
  const argv = [node, appRunner, "app-runner", target]
    .map(tomlString)
    .join(", ");
  return `target.${target}.runner=[${argv}]`;
}

function cargoFeatures(args) {
  const features = new Set();
  for (let index = 0; index < args.length; index += 1) {
    let value;
    if (args[index] === "--features") {
      value = args[index + 1];
      index += 1;
    } else if (args[index].startsWith("--features=")) {
      value = args[index].slice("--features=".length);
    }
    if (!value) continue;
    for (const feature of value.split(/[,\s]+/).filter(Boolean)) {
      features.add(feature);
    }
  }
  return features;
}

function validateCargoArguments(args, expectedTarget) {
  if (args[0] !== "run") {
    fail("Tauri must invoke the signed runner through cargo run");
  }
  if (
    args.some(
      (argument) => argument === "--config" || argument.startsWith("--config="),
    )
  ) {
    fail("Tauri Cargo arguments must not override the project-owned runner");
  }
  const targetIndexes = [];
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === "--target") targetIndexes.push(index);
  }
  if (
    targetIndexes.length !== 1 ||
    args[targetIndexes[0] + 1] !== expectedTarget
  ) {
    fail("Tauri Cargo target drifted");
  }
  const delimiter = args.indexOf("--");
  if (delimiter >= 0 && delimiter !== args.length - 1) {
    fail("signed development does not accept forwarded application arguments");
  }
  const features = cargoFeatures(args);
  if (!features.has("macos-privileged-client")) {
    fail("Tauri Cargo invocation did not enable the privileged client");
  }
}

function main() {
  switch (process.platform) {
    case "darwin":
      break;
    default:
      throw new Error(
        "macos-signed-dev-cargo: the signed Cargo runner is macOS-only",
      );
  }
  const target = process.env.FYAGENT_SIGNED_DEV_TARGET;
  if (!target || !/^(?:aarch64|x86_64)-apple-darwin$/.test(target)) {
    fail("FYAGENT_SIGNED_DEV_TARGET is invalid");
  }
  if (process.env.FYAGENT_MACOS_SYSTEM_COMMIT_MODE !== "development") {
    fail("macOS system-commit build mode is not development");
  }
  const cargo = existingFile(
    process.env.FYAGENT_SIGNED_DEV_CARGO,
    "project-owned Cargo executable",
    { allowSymlink: true },
  );
  const node = existingFile(
    process.env.FYAGENT_SIGNED_DEV_NODE,
    "project-owned Node executable",
    { allowSymlink: true },
  );
  const appRunner = existingFile(
    process.env.FYAGENT_SIGNED_DEV_APP_RUNNER,
    "signed app runner",
  );
  existingFile(
    process.env.FYAGENT_PRIVILEGED_CLIENT_DYLIB,
    "privileged client dylib",
  );
  existingFile(
    process.env.FYAGENT_PRIVILEGED_MANIFEST,
    "privileged artifact manifest",
  );

  const cargoArguments = process.argv.slice(2);
  validateCargoArguments(cargoArguments, target);
  const config = targetRunnerConfig(target, node, appRunner);
  process.execve(
    cargo,
    [cargo, "--config", config, ...cargoArguments],
    process.env,
  );
}

try {
  main();
} catch (error) {
  process.stderr.write(
    `${error instanceof Error ? error.message : String(error)}\n`,
  );
  process.exit(1);
}
