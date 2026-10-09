import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const syntheticPosixHome = ["", "home", "local-developer", "project"].join("/");

const concretePosixHome = /\/(?:Users|home)\/(?!<)[^/\s`"'\\]+(?:\/|$)/u;
const concreteWindowsHome =
  /\b[A-Za-z]:[\\/]Users[\\/](?!<)[^\\/\s`"']+(?:[\\/]|$)/u;

function trackedDocumentationFiles(): string[] {
  const result = spawnSync(
    "git",
    [
      "ls-files",
      "-z",
      "--",
      ":(glob)**/*.md",
      ":(glob).trellis/**/*.json",
      ":(glob).trellis/**/*.jsonl",
    ],
    {
      cwd: ROOT,
      encoding: "buffer",
      shell: false,
      windowsHide: true,
    },
  );
  if (result.status !== 0 || result.error !== undefined) {
    throw new Error("failed to enumerate tracked documentation files");
  }
  return result.stdout.toString("utf8").split("\0").filter(Boolean).sort();
}

async function readTrackedDocumentationFile(
  file: string,
  root = ROOT,
): Promise<string> {
  try {
    return await fs.promises.readFile(path.join(root, file), "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }

  const result = spawnSync("git", ["show", `:${file}`], {
    cwd: root,
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.status !== 0 || result.error !== undefined) {
    throw new Error(`failed to read tracked documentation file: ${file}`);
  }
  return result.stdout;
}

async function trackedDocumentationViolations(
  files: string[],
  root = ROOT,
  readSource = readTrackedDocumentationFile,
): Promise<string[]> {
  const violationsByFile: string[][] = new Array(files.length);
  let nextIndex = 0;
  // Keep the same bounded I/O limit without waiting for the slowest read in
  // every batch. Each available reader scans fresh bytes from the next file.
  await Promise.all(
    Array.from({ length: Math.min(32, files.length) }, async () => {
      while (nextIndex < files.length) {
        const index = nextIndex++;
        const file = files[index];
        const source = await readSource(file, root);
        violationsByFile[index] = concreteWorkstationHomeLocations(
          file,
          source,
        );
      }
    }),
  );
  return violationsByFile.flat();
}

function concreteWorkstationHomeLocations(
  file: string,
  source: string,
): string[] {
  return source
    .split(/\r?\n/u)
    .flatMap((line, index) =>
      concretePosixHome.test(line) || concreteWindowsHome.test(line)
        ? [`${file}:${index + 1}`]
        : [],
    );
}

describe("repository workstation path privacy contract", () => {
  it("distinguishes semantic home placeholders from concrete workstation paths", () => {
    expect(
      concreteWorkstationHomeLocations(
        "fixture.md",
        [
          "Use ~/project or $HOME/project.",
          "macOS: /Users/<username>/project",
          String.raw`Windows: C:\Users\<username>\project`,
        ].join("\n"),
      ),
    ).toEqual([]);

    expect(
      concreteWorkstationHomeLocations(
        "fixture.md",
        [
          "macOS: /Users/local-developer/project",
          String.raw`Windows: C:\Users\local-developer\project`,
        ].join("\n"),
      ),
    ).toEqual(["fixture.md:1", "fixture.md:2"]);
  });

  it("keeps concrete device-local user-home paths out of tracked docs and Trellis artifacts", async () => {
    const files = trackedDocumentationFiles();
    expect(
      files.length,
      "tracked documentation scan must be nonempty",
    ).toBeGreaterThan(0);
    const violations = await trackedDocumentationViolations(files);

    expect(
      violations,
      `Concrete workstation user-home paths must use semantic placeholders; locations only:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("checks uncommitted worktree text and falls back to the index only for missing files", async () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-privacy-fixture-"),
    );
    try {
      expect(
        spawnSync("git", ["init", "--quiet"], { cwd: root, windowsHide: true })
          .status,
      ).toBe(0);
      fs.writeFileSync(
        path.join(root, "changed.md"),
        "Use /Users/<username>/project",
      );
      fs.writeFileSync(path.join(root, "missing.md"), syntheticPosixHome);
      expect(
        spawnSync("git", ["add", "--all"], { cwd: root, windowsHide: true })
          .status,
      ).toBe(0);
      fs.writeFileSync(
        path.join(root, "changed.md"),
        String.raw`C:\Users\local-developer\project`,
      );
      fs.rmSync(path.join(root, "missing.md"));

      expect(
        await trackedDocumentationViolations(
          ["changed.md", "missing.md"],
          root,
        ),
      ).toEqual(["changed.md:1", "missing.md:1"]);
      await expect(
        readTrackedDocumentationFile("absent.md", root),
      ).rejects.toThrow("failed to read tracked documentation file");
    } finally {
      if (
        path.dirname(root) === path.resolve(os.tmpdir()) &&
        path.basename(root).startsWith("fyagent-privacy-fixture-")
      ) {
        fs.rmSync(root, { recursive: true, force: true });
      }
    }
  });

  it("overlaps bounded document reads while checking every file and propagating read failures", async () => {
    const files = Array.from(
      { length: 65 },
      (_, index) => `fixture-${index}.md`,
    );
    const read: string[] = [];
    let active = 0;
    let peak = 0;
    let releaseFirstRead: () => void = () => {};
    const slowFirstRead = new Promise<void>((resolve) => {
      releaseFirstRead = resolve;
    });
    const violations = await trackedDocumentationViolations(
      files,
      ROOT,
      async (file) => {
        read.push(file);
        active += 1;
        peak = Math.max(peak, active);
        // A fixed 32-file barrier cannot start this later file until the
        // blocked first read finishes. Available readers must keep progressing.
        if (file === files[39]) releaseFirstRead();
        if (file === files[0]) await slowFirstRead;
        await new Promise<void>((resolve) => setTimeout(resolve, 4));
        active -= 1;
        return syntheticPosixHome;
      },
    );
    expect(read).toEqual(files);
    expect(peak).toBeGreaterThan(1);
    expect(peak).toBeLessThanOrEqual(32);
    expect(violations).toEqual(files.map((file) => `${file}:1`));
    await expect(
      trackedDocumentationViolations(["unreadable.md"], ROOT, async () => {
        throw new Error("controlled document read failure");
      }),
    ).rejects.toThrow("controlled document read failure");
  });
});
