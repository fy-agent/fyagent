import assert from "node:assert/strict";
import cp from "node:child_process";
import { syncBuiltinESMExports } from "node:module";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { test } from "node:test";

const ROOT = path.resolve(import.meta.dirname, "..");

async function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-upgrade-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.cpSync(path.join(ROOT, "scripts"), path.join(root, "scripts"), {
    recursive: true,
  });
  fs.symlinkSync(
    path.join(ROOT, "node_modules"),
    path.join(root, "node_modules"),
    "junction",
  );
  for (const file of [
    ".node-version",
    "package.json",
    "rust-toolchain.toml",
    ".python-version",
    "mise.toml",
    "mise.lock",
    "uv.lock",
    "pyproject.toml",
  ]) {
    fs.copyFileSync(path.join(ROOT, file), path.join(root, file));
  }
  const { toolchainUpdate } = await import(
    pathToFileURL(path.join(root, "scripts/tasks/maintenance.mjs"))
  );
  const { validateLockfile } = await import(
    pathToFileURL(path.join(root, "scripts/tasks/lockfile-check.mjs"))
  );
  const { readToolchainContract } = await import(
    pathToFileURL(path.join(root, "scripts/ci/verify-toolchain.mjs"))
  );
  return { root, toolchainUpdate, validateLockfile, readToolchainContract };
}

function nextPatch(version) {
  const parts = version.split(".");
  parts[2] = String(Number(parts[2]) + 1);
  return parts.join(".");
}

function updateLock(root, tool, version) {
  const file = path.join(root, "mise.lock");
  const source = fs.readFileSync(file, "utf8");
  const pattern = new RegExp(
    `(\\[\\[tools\\.${tool}\\]\\][\\s\\S]*?\\nversion = ")[^"]+(")`,
  );
  assert.match(source, pattern);
  fs.writeFileSync(file, source.replace(pattern, `$1${version}$2`));
}

for (const tool of ["node", "pnpm", "rust"]) {
  test(`${tool} patch upgrade passes the real checker without rollback`, async (t) => {
    const f = await fixture(t);
    const version = nextPatch(f.readToolchainContract(f.root)[tool]);
    const calls = [];
    f.toolchainUpdate(tool, {
      version,
      apply: true,
      run(command, args) {
        calls.push([command, args]);
        if (command === "mise") updateLock(f.root, tool, version);
        else {
          assert.equal(command, "node");
          assert.deepEqual(args, ["scripts/tasks/lockfile-check.mjs"]);
          assert.equal(f.validateLockfile().tools[tool], version);
        }
      },
    });
    assert.equal(calls.length, 2);
    assert.equal(f.readToolchainContract(f.root)[tool], version);
    assert.equal(f.validateLockfile().tools[tool], version);
  });

  test(`${tool} lock-only drift is rejected`, async (t) => {
    const f = await fixture(t);
    updateLock(f.root, tool, nextPatch(f.readToolchainContract(f.root)[tool]));
    assert.throws(() => f.validateLockfile(), /lock version drifted/);
  });
}

for (const stage of ["mise", "checker"]) {
  test(`failed ${stage} restores both authoritative file and lock byte-for-byte`, async (t) => {
    const f = await fixture(t);
    const files = [".node-version", "mise.lock"];
    const before = files.map((file) =>
      fs.readFileSync(path.join(f.root, file)),
    );
    const version = nextPatch(f.readToolchainContract(f.root).node);
    assert.throws(
      () =>
        f.toolchainUpdate("node", {
          version,
          apply: true,
          run(command) {
            if (command === "mise") {
              updateLock(f.root, "node", version);
              if (stage === "mise") throw new Error("injected lock failure");
            } else {
              updateLock(f.root, "node", nextPatch(version));
              f.validateLockfile();
            }
          },
        }),
      /injected lock failure|lock version drifted/,
    );
    files.forEach((file, i) =>
      assert.deepEqual(fs.readFileSync(path.join(f.root, file)), before[i]),
    );
    assert.equal(f.validateLockfile().ok, true);
  });
}

// Exercise the ownership checker with real filesystem paths and injected process output.
test("ownership checks follow upgraded facts and reject actual version mismatch", async (t) => {
  const f = await fixture(t);
  for (const tool of ["node", "pnpm", "rust"]) {
    const version = nextPatch(f.readToolchainContract(f.root)[tool]);
    f.toolchainUpdate(tool, {
      version,
      apply: true,
      run(command) {
        if (command === "mise") updateLock(f.root, tool, version);
        else f.validateLockfile();
      },
    });
  }
  const facts = f.readToolchainContract(f.root);
  const bin = path.join(f.root, "tools", facts.rust, "bin");
  fs.mkdirSync(bin, { recursive: true });
  for (const tool of ["node", "pnpm", "uv", "rustc"])
    fs.writeFileSync(path.join(bin, tool), "");
  fs.mkdirSync(path.join(f.root, ".venv/bin"), { recursive: true });
  fs.writeFileSync(path.join(f.root, ".venv/bin/python"), "");
  const preload = path.join(f.root, "runner.mjs");
  fs.writeFileSync(
    preload,
    `
import cp from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
const facts = ${JSON.stringify(facts)};
const root = ${JSON.stringify(f.root)};
const bin = ${JSON.stringify(bin)};
cp.spawnSync = (command, args) => {
  let stdout = '';
  if (command === 'node') stdout = 'v' + (process.env.BAD_NODE || facts.node);
  else if (command === 'pnpm') stdout = process.env.BAD_PNPM || facts.pnpm;
  else if (command === 'uv') stdout = args[0] === '--version' ? 'uv ' + facts.uv : args[0] === 'python' ? bin + '/python' : args[0] === 'run' ? facts.python : '';
  else if (command.endsWith('/python')) stdout = facts.python;
  else if (command === 'rustc') stdout = args[0] === '--version' ? 'rustc ' + (process.env.BAD_RUST || facts.rust) + ' (fixture)' : root + '/tools/' + facts.rust;
  else if (command === 'rustup') stdout = args[0] === 'component' ? 'rustfmt-fixture\\nclippy-fixture' : facts.rust + '-fixture';
  else if (command === 'mise') {
    if (args[0] === 'which') stdout = bin + '/' + args[1];
    else if (args[0] === 'where') stdout = root + '/tools/' + facts.rust;
    else if (args[0] === 'current') stdout = facts.rust;
    else if (args[0] === 'config') stdout = JSON.stringify(['mise.toml', '.node-version', 'package.json', 'rust-toolchain.toml'].map(file => ({path: root + '/' + file, tools: file === 'mise.toml' ? ['uv'] : []})));
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
      assert.match(
        check.detail,
        tool === "rust" ? /rustc/ : /version: expected/,
      );
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
});
