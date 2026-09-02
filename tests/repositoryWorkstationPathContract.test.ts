import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");

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
  return result.stdout
    .toString("utf8")
    .split("\0")
    .filter(Boolean)
    .sort();
}

function readTrackedDocumentationFile(file: string): string {
  try {
    return fs.readFileSync(path.join(ROOT, file), "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }

  const result = spawnSync("git", ["show", `:${file}`], {
    cwd: ROOT,
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.status !== 0 || result.error !== undefined) {
    throw new Error(`failed to read tracked documentation file: ${file}`);
  }
  return result.stdout;
}

function concreteWorkstationHomeLocations(
  file: string,
  source: string,
): string[] {
  return source.split(/\r?\n/u).flatMap((line, index) =>
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

  it("keeps concrete device-local user-home paths out of tracked docs and Trellis artifacts", () => {
    const violations = trackedDocumentationFiles().flatMap((file) =>
      concreteWorkstationHomeLocations(
        file,
        readTrackedDocumentationFile(file),
      ),
    );

    expect(
      violations,
      `Concrete workstation user-home paths must use semantic placeholders; locations only:\n${violations.join("\n")}`,
    ).toEqual([]);
  });
});
