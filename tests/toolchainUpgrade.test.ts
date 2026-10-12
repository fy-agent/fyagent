import assert from "node:assert/strict";
import cp from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { test, type TestContext } from "vitest";

const ROOT = path.resolve(__dirname, "..");

async function fixture(t: TestContext) {
  const root = fs.mkdtempSync(path.join(ROOT, ".fyagent-upgrade-"));
  t.onTestFinished(() => fs.rmSync(root, { recursive: true, force: true }));
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
    /* @vite-ignore */ pathToFileURL(
      path.join(root, "scripts/tasks/maintenance.mjs"),
    ).href
  );
  const { validateLockfile } = await import(
    /* @vite-ignore */ pathToFileURL(
      path.join(root, "scripts/tasks/lockfile-check.mjs"),
    ).href
  );
  const { readToolchainContract } = await import(
    /* @vite-ignore */ pathToFileURL(
      path.join(root, "scripts/ci/verify-toolchain.mjs"),
    ).href
  );
  return { root, toolchainUpdate, validateLockfile, readToolchainContract };
}

function nextPatch(version: string) {
  const parts = version.split(".");
  parts[2] = String(Number(parts[2]) + 1);
  return parts.join(".");
}

function updateLock(root: string, tool: string, version: string) {
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
    const calls: [string, string[]][] = [];
    f.toolchainUpdate(tool, {
      version,
      apply: true,
      run(command: string, args: string[]) {
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
          run(command: string) {
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
      run(command: string) {
        if (command === "mise") updateLock(f.root, tool, version);
        else f.validateLockfile();
      },
    });
  }
  // Native ESM is required for the existing syncBuiltinESMExports injection.
  const result = cp.spawnSync(
    process.execPath,
    [path.join(ROOT, "tests/fixtures/toolchain-ownership.mjs"), f.root],
    { cwd: f.root, encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr || result.stdout);
});

test("malformed version declaration still produces the JSON check report", async (t) => {
  const f = await fixture(t);
  fs.writeFileSync(path.join(f.root, ".node-version"), "broken\n");
  const result = cp.spawnSync(
    process.execPath,
    ["scripts/tasks/toolchain-check.mjs", "--json"],
    { cwd: f.root, encoding: "utf8" },
  );
  assert.equal(result.status, 1);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, false);
  const check = report.checks.find(
    (check: { name: string }) => check.name === "standard version facts",
  );
  assert.equal(check.ok, false);
  assert.match(check.detail, /\.node-version must contain one exact stable/);
});

test("Python declaration must satisfy requires-python while allowing patch upgrades", async (t) => {
  const f = await fixture(t);
  fs.writeFileSync(
    path.join(f.root, ".python-version"),
    nextPatch(f.readToolchainContract(f.root).python),
  );
  assert.equal(f.validateLockfile().ok, true);
  for (const version of ["3.13.0", "3.15.0"]) {
    fs.writeFileSync(path.join(f.root, ".python-version"), version);
    assert.throws(
      () => f.validateLockfile(),
      /\.python-version .*requires-python/,
    );
  }
});
