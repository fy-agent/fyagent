import assert from "node:assert/strict";
import cp from "node:child_process";
import { syncBuiltinESMExports } from "node:module";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";

function nextPatch(version) {
  const parts = version.split(".");
  parts[2] = String(Number(parts[2]) + 1);
  return parts.join(".");
}

const f = {
  root: process.argv[2],
  readToolchainContract: (
    await import(
      pathToFileURL(
        path.join(process.argv[2], "scripts/ci/verify-toolchain.mjs"),
      )
    )
  ).readToolchainContract,
};
const facts = f.readToolchainContract(f.root);
const isWindows = os.platform() === "win32";
const extensions = isWindows
  ? (process.env.PATHEXT ?? ".COM;.EXE;.BAT;.CMD").split(";")
  : [""];
const bin = path.join(f.root, "tools", facts.rust, "bin");
fs.mkdirSync(bin, { recursive: true });
const toolPaths = {};
for (const tool of ["node", "pnpm", "uv", "rustc"]) {
  for (const extension of extensions)
    fs.writeFileSync(path.join(bin, `${tool}${extension}`), "");
  toolPaths[tool] = path.join(bin, `${tool}${extensions[0]}`);
}
const venvPython = isWindows
  ? path.join(f.root, ".venv", "Scripts", "python.exe")
  : path.join(f.root, ".venv", "bin", "python");
fs.mkdirSync(path.dirname(venvPython), { recursive: true });
fs.writeFileSync(venvPython, "");
const preload = path.join(f.root, "runner.mjs");
fs.writeFileSync(
  preload,
  `
import cp from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
import path from 'node:path';
const facts = ${JSON.stringify(facts)};
const root = ${JSON.stringify(f.root)};
const bin = ${JSON.stringify(bin)};
const toolPaths = ${JSON.stringify(toolPaths)};
const venvPython = ${JSON.stringify(venvPython)};
cp.spawnSync = (command, args) => {
  const tool = path.basename(command).replace(/\\.(?:exe|cmd|bat|com)$/i, '');
  let stdout = '';
  if (tool === 'node') stdout = 'v' + (process.env.BAD_NODE || facts.node);
  else if (tool === 'pnpm') stdout = process.env.BAD_PNPM || facts.pnpm;
  else if (tool === 'uv') stdout = args[0] === '--version' ? 'uv ' + facts.uv : args[0] === 'python' ? path.join(bin, ${JSON.stringify(isWindows ? "python.exe" : "python")}) : args[0] === 'run' ? facts.python : '';
  else if (command === venvPython) stdout = facts.python;
  else if (tool === 'rustc') stdout = args[0] === '--version' ? 'rustc ' + (process.env.BAD_RUST || facts.rust) + ' (fixture)' : path.join(root, 'tools', facts.rust);
  else if (tool === 'rustup') stdout = args[0] === 'component' ? 'rustfmt-fixture\\nclippy-fixture' : facts.rust + '-fixture';
  else if (tool === 'mise') {
    if (args[0] === 'which') stdout = toolPaths[args[1]];
    else if (args[0] === 'where') stdout = path.join(root, 'tools', facts.rust);
    else if (args[0] === 'current') stdout = facts.rust;
    else if (args[0] === 'config') stdout = JSON.stringify(['mise.toml', '.node-version', 'package.json', 'rust-toolchain.toml'].map(file => ({path: path.join(root, file), tools: file === 'mise.toml' ? ['uv'] : []})));
    else if (args[1] === 'ls') stdout = JSON.stringify([{name: 'env:check'}]);
  } else throw new Error('Unexpected command ' + command);
  return {status: 0, stdout, stderr: ''};
};
syncBuiltinESMExports();
`,
);
const original = {
  spawnSync: cp.spawnSync,
  cwd: process.cwd(),
  env: { ...process.env },
  argv: [...process.argv],
  log: console.log,
  exitCode: process.exitCode,
};
const reports = [];
try {
  process.chdir(f.root);
  process.env.PATH = bin;
  process.argv = [process.execPath, "fixture", "--json"];
  console.log = (value) => reports.push(JSON.parse(value));
  await import(pathToFileURL(preload));
  await import(
    pathToFileURL(path.join(f.root, "scripts/tasks/toolchain-check.mjs"))
  );
  assert.equal(reports[0].ok, true, JSON.stringify(reports[0]));
  for (const [tool, envName, checkName] of [
    ["node", "BAD_NODE", "Node ownership"],
    ["pnpm", "BAD_PNPM", "pnpm ownership"],
    ["rust", "BAD_RUST", "Rust toolchain and components"],
  ]) {
    process.env[envName] = nextPatch(facts[tool]);
    await import(
      pathToFileURL(path.join(f.root, "scripts/tasks/toolchain-check.mjs")) +
        `?mismatch=${tool}`
    );
    assert.equal(process.exitCode, 1);
    const check = reports
      .at(-1)
      .checks.find((check) => check.name === checkName);
    assert.equal(check.ok, false);
    assert.match(check.detail, tool === "rust" ? /rustc/ : /version: expected/);
    delete process.env[envName];
    process.exitCode = original.exitCode;
  }
} finally {
  cp.spawnSync = original.spawnSync;
  syncBuiltinESMExports();
  process.chdir(original.cwd);
  process.env = original.env;
  process.argv = original.argv;
  process.exitCode = original.exitCode;
  console.log = original.log;
}
