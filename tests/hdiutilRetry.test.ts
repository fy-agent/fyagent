import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const RETRY_HDIUTIL = path.join(ROOT, "scripts", "release", "retry-hdiutil.sh");
const temporaryRoots: string[] = [];

function resolveBashExecutable(): string {
  switch (process.platform) {
    case "darwin":
      return "bash";
    case "win32":
      break;
    default:
      throw new Error(`Unsupported test host: ${process.platform}`);
  }

  const gitExecPath = spawnSync("git", ["--exec-path"], {
    encoding: "utf8",
    windowsHide: true,
  });
  if (gitExecPath.status !== 0) {
    throw new Error(`git --exec-path failed: ${gitExecPath.stderr}`);
  }
  const gitRoot = path.resolve(gitExecPath.stdout.trim(), "..", "..", "..");
  for (const candidate of [
    path.join(gitRoot, "bin", "bash.exe"),
    path.join(gitRoot, "usr", "bin", "bash.exe"),
  ]) {
    if (fs.existsSync(candidate)) return candidate;
  }
  throw new Error(`Git Bash was not found below ${gitRoot}`);
}

const BASH_RUNNER = `
fake_bin="$1"
retry_script="$2"
shift 2

if command -v cygpath >/dev/null 2>&1; then
  fake_bin="$(cygpath -u "$fake_bin")"
  retry_script="$(cygpath -u "$retry_script")"
fi

export PATH="$fake_bin:$PATH"
exec "$retry_script" "$@"
`;

function createFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-hdiutil-"));
  temporaryRoots.push(root);
  const binRoot = path.join(root, "bin");
  const callRoot = path.join(root, "calls");
  const outputPath = path.join(root, "release assets", "FyAgent test.dmg");
  const statePath = path.join(root, "attempt.txt");
  const sleepLog = path.join(root, "sleep.log");
  fs.mkdirSync(binRoot);
  fs.mkdirSync(callRoot);
  fs.mkdirSync(path.dirname(outputPath));

  fs.writeFileSync(
    path.join(binRoot, "hdiutil"),
    `#!/usr/bin/env bash
set -euo pipefail

attempt=0
if [ -f "$FYAGENT_FAKE_STATE" ]; then
  attempt="$(<"$FYAGENT_FAKE_STATE")"
fi
attempt=$((attempt + 1))
printf '%s\n' "$attempt" > "$FYAGENT_FAKE_STATE"

args_file="$FYAGENT_FAKE_CALL_ROOT/args.$attempt"
: > "$args_file"
for argument in "$@"; do
  printf '%s\n' "$argument" >> "$args_file"
done

operation="$1"
if [ "$operation" = verify ]; then
  if [ ! -f "$FYAGENT_FAKE_OUTPUT" ]; then
    echo 'verify input disappeared' >&2
    exit 96
  fi
elif [ -e "$FYAGENT_FAKE_OUTPUT" ]; then
  echo 'stale partial output reached hdiutil' >&2
  exit 97
fi

if [ "$operation" != verify ]; then
  : > "$FYAGENT_FAKE_OUTPUT"
fi
if [ "$attempt" -le "$FYAGENT_FAKE_BUSY_FAILURES" ]; then
  echo "hdiutil: $operation failed - $FYAGENT_FAKE_TRANSIENT_DIAGNOSTIC" >&2
  exit 73
fi
if [ "$FYAGENT_FAKE_MODE" = fail ]; then
  echo 'hdiutil: create failed - permission denied' >&2
  exit 42
fi

echo 'created: fake disk image'
`,
    { mode: 0o755 },
  );
  fs.writeFileSync(
    path.join(binRoot, "sleep"),
    `#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$1" >> "$FYAGENT_FAKE_SLEEP_LOG"
`,
    { mode: 0o755 },
  );

  function run(
    busyFailures: number,
    mode: "fail" | "succeed",
    hdiutilArguments: string[],
    transientDiagnostic = "Resource busy",
  ) {
    fs.writeFileSync(outputPath, "stale output");
    return spawnSync(
      resolveBashExecutable(),
      [
        "-c",
        BASH_RUNNER,
        "fyagent-hdiutil-test",
        binRoot,
        RETRY_HDIUTIL,
        outputPath,
        "--",
        ...hdiutilArguments,
      ],
      {
        encoding: "utf8",
        windowsHide: true,
        env: {
          ...process.env,
          FYAGENT_FAKE_BUSY_FAILURES: String(busyFailures),
          FYAGENT_FAKE_CALL_ROOT: callRoot,
          FYAGENT_FAKE_MODE: mode,
          FYAGENT_FAKE_OUTPUT: outputPath,
          FYAGENT_FAKE_SLEEP_LOG: sleepLog,
          FYAGENT_FAKE_STATE: statePath,
          FYAGENT_FAKE_TRANSIENT_DIAGNOSTIC: transientDiagnostic,
        },
      },
    );
  }

  function calls(): string[][] {
    if (!fs.existsSync(statePath)) return [];
    const count = Number(fs.readFileSync(statePath, "utf8").trim());
    return Array.from({ length: count }, (_, index) =>
      fs
        .readFileSync(path.join(callRoot, `args.${index + 1}`), "utf8")
        .trimEnd()
        .split("\n"),
    );
  }

  function sleeps(): string[] {
    return fs.existsSync(sleepLog)
      ? fs.readFileSync(sleepLog, "utf8").trim().split("\n")
      : [];
  }

  return { calls, outputPath, run, sleeps };
}

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) {
    fs.rmSync(root, { force: true, recursive: true });
  }
});

describe("hdiutil Resource busy retry", () => {
  it("removes partial output and succeeds after two Resource busy failures", () => {
    const fixture = createFixture();
    const args = [
      "create",
      "-volname",
      "FyAgent",
      "-srcfolder",
      "/tmp/stage",
      "-ov",
      "-format",
      "UDZO",
      fixture.outputPath,
    ];

    const result = fixture.run(2, "succeed", args);

    expect(result.status, result.stderr).toBe(0);
    expect(fixture.calls()).toEqual([args, args, args]);
    expect(fixture.sleeps()).toEqual(["2", "4"]);
    expect(fs.existsSync(fixture.outputPath)).toBe(true);
  });

  it("returns the original status immediately for a non-Resource busy error", () => {
    const fixture = createFixture();
    const args = ["create", "-format", "UDZO", fixture.outputPath];

    const result = fixture.run(0, "fail", args);

    expect(result.status, result.stderr).toBe(42);
    expect(fixture.calls()).toEqual([args]);
    expect(fixture.sleeps()).toEqual([]);
    expect(fs.existsSync(fixture.outputPath)).toBe(false);
  });

  it("preserves a completed image while retrying transient verify failures", () => {
    const fixture = createFixture();
    const args = ["verify", fixture.outputPath];

    const result = fixture.run(
      2,
      "succeed",
      args,
      "Resource temporarily unavailable",
    );

    expect(result.status, result.stderr).toBe(0);
    expect(fixture.calls()).toEqual([args, args, args]);
    expect(fixture.sleeps()).toEqual(["2", "4"]);
    expect(fs.readFileSync(fixture.outputPath, "utf8")).toBe("stale output");
  });

  it("stops after five Resource busy failures with bounded backoff", () => {
    const fixture = createFixture();
    const args = ["create", "-format", "UDZO", fixture.outputPath];

    const result = fixture.run(5, "succeed", args);

    expect(result.status, result.stderr).toBe(73);
    expect(fixture.calls()).toEqual([args, args, args, args, args]);
    expect(fixture.sleeps()).toEqual(["2", "4", "8", "16"]);
    expect(fs.existsSync(fixture.outputPath)).toBe(false);
  });

  it("preserves every hdiutil argument without shell re-parsing", () => {
    const fixture = createFixture();
    const args = [
      "create",
      "-volname",
      "Fy Agent * $(unchanged)",
      "-srcfolder",
      "/tmp/stage with spaces",
      "--opaque=value with spaces",
      fixture.outputPath,
    ];

    const result = fixture.run(0, "succeed", args);

    expect(result.status, result.stderr).toBe(0);
    expect(fixture.calls()).toEqual([args]);
    expect(fixture.sleeps()).toEqual([]);
  });
});
