import { describe, expect, it, vi } from "vitest";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import * as releaseCheckModule from "../scripts/tasks/release-check.mjs";

type ReleaseCheck = readonly [id: string, command: string, args: string[]];

const parseReleaseCheckMode = releaseCheckModule.parseReleaseCheckMode as (
  args: string[],
) => boolean;
const releaseCheckPlan = releaseCheckModule.releaseCheckPlan as (
  ciMode: boolean,
) => ReleaseCheck[];
const runReleaseChecks = releaseCheckModule.runReleaseChecks as (
  ciMode: boolean,
  execute?: (command: string, args: string[]) => void,
) => void;

describe("release check diagnostic aggregation", () => {
  it("keeps CI-only and local-only diagnostics explicit", () => {
    expect(parseReleaseCheckMode(["--ci"])).toBe(true);
    expect(parseReleaseCheckMode([])).toBe(false);
    expect(() => parseReleaseCheckMode(["--unknown"])).toThrow(
      "Usage: release-check.mjs [--ci]",
    );

    const ciIds = releaseCheckPlan(true).map(([id]) => id);
    expect(ciIds).toEqual([
      "version",
      "lockfile",
      "dep0040",
      "task-docs",
      "windows-nsis-contract",
      "supported-platform",
      "contract-tests",
    ]);
    expect(releaseCheckPlan(false).map(([id]) => id)).toEqual([
      "version",
      "lockfile",
      "dep0040",
      "task-contract",
      "task-docs",
      "windows-nsis-contract",
      "supported-platform",
      "contract-tests",
      "native-fetch",
    ]);
    const contractTests = releaseCheckPlan(true).find(
      ([id]) => id === "contract-tests",
    );
    expect(contractTests?.[2]).toContain("tests/ciStepOutcomes.test.ts");
    expect(contractTests?.[2]).toContain(
      "tests/releaseCheckAggregation.test.ts",
    );
    expect(releaseCheckPlan(true)).toContainEqual([
      "supported-platform",
      "node",
      ["scripts/tasks/supported-platform-check.mjs"],
    ]);
  });

  it("runs every independent diagnostic before returning an aggregate failure", () => {
    const calls: string[] = [];
    const log = vi.spyOn(console, "log").mockImplementation(() => undefined);
    const error = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    expect(() =>
      runReleaseChecks(true, (command, args) => {
        const id = `${command} ${args.join(" ")}`;
        calls.push(id);
        if (calls.length === 1 || calls.length === 5) {
          throw new Error(`fixture failure ${calls.length}`);
        }
      }),
    ).toThrow("2 release diagnostic(s) failed: version, windows-nsis-contract");

    expect(calls).toHaveLength(releaseCheckPlan(true).length);
    expect(error.mock.calls.map(([line]) => line)).toEqual([
      "[release-check] version failed: fixture failure 1",
      "[release-check] windows-nsis-contract failed: fixture failure 5",
    ]);

    log.mockRestore();
    error.mockRestore();
  });
});
