import fs from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
// @ts-expect-error The task runner executes this JavaScript helper directly.
import { writeFilesAtomically } from "../scripts/tasks/lib.mjs";

const ROOT = path.resolve(__dirname, "..");
const FIXTURE_ROOT = path.join(ROOT, ".fyagent", "test-fixtures");

function withFixture(run: (fixture: string) => void) {
  fs.mkdirSync(FIXTURE_ROOT, { recursive: true });
  const fixture = fs.mkdtempSync(
    path.join(FIXTURE_ROOT, "atomic-writer-contract-"),
  );
  try {
    run(fixture);
  } finally {
    fs.rmSync(fixture, { recursive: true, force: true });
  }
}

function relative(file: string) {
  return path.relative(ROOT, file);
}

function temporaryFiles(fixture: string) {
  return fs
    .readdirSync(fixture)
    .filter((entry) => entry.includes(".fyagent-task-"));
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("shared atomic task writer", () => {
  it.runIf(process.platform === "darwin")(
    "uses unique same-directory temporary files and preserves target modes",
    () => {
      withFixture((fixture) => {
        const first = path.join(fixture, "first.txt");
        const second = path.join(fixture, "second.txt");
        fs.writeFileSync(first, "first original\n", { mode: 0o751 });
        fs.writeFileSync(second, "second original\n", { mode: 0o640 });
        fs.chmodSync(first, 0o751);
        fs.chmodSync(second, 0o640);

        const originalRename = fs.renameSync.bind(fs);
        const renames: Array<{ source: string; destination: string }> = [];
        vi.spyOn(fs, "renameSync").mockImplementation((source, destination) => {
          renames.push({
            source: path.resolve(String(source)),
            destination: path.resolve(String(destination)),
          });
          originalRename(source, destination);
        });

        writeFilesAtomically([
          [relative(first), "first updated\n"],
          [relative(second), "second updated\n"],
        ]);

        expect(fs.readFileSync(first, "utf8")).toBe("first updated\n");
        expect(fs.readFileSync(second, "utf8")).toBe("second updated\n");
        expect(fs.statSync(first).mode & 0o7777).toBe(0o751);
        expect(fs.statSync(second).mode & 0o7777).toBe(0o640);
        expect(new Set(renames.map(({ source }) => source)).size).toBe(2);
        for (const { source, destination } of renames) {
          expect(path.dirname(source)).toBe(path.dirname(destination));
          expect(path.basename(source)).toMatch(
            new RegExp(
              `^\\.${path.basename(destination)}\\.fyagent-task-\\d+-[0-9a-f-]+\\.tmp$`,
              "u",
            ),
          );
        }
        expect(temporaryFiles(fixture)).toEqual([]);
      });
    },
  );

  it("rolls back only replaced targets instead of rewriting a failed destination", () => {
    withFixture((fixture) => {
      const first = path.join(fixture, "first.txt");
      const second = path.join(fixture, "second.txt");
      fs.writeFileSync(first, "first original\n");
      fs.writeFileSync(second, "second original\n");

      const primaryError = new Error("injected forward rename failure");
      const originalRename = fs.renameSync.bind(fs);
      let renameCalls = 0;
      vi.spyOn(fs, "renameSync").mockImplementation((source, destination) => {
        renameCalls += 1;
        if (renameCalls === 2) {
          fs.writeFileSync(second, "concurrent editor content\n");
          throw primaryError;
        }
        originalRename(source, destination);
      });

      let thrown: unknown;
      try {
        writeFilesAtomically([
          [relative(first), "first updated\n"],
          [relative(second), "second updated\n"],
        ]);
      } catch (error) {
        thrown = error;
      }

      expect(thrown).toBe(primaryError);
      expect(renameCalls).toBe(3);
      expect(fs.readFileSync(first, "utf8")).toBe("first original\n");
      expect(fs.readFileSync(second, "utf8")).toBe(
        "concurrent editor content\n",
      );
      expect(temporaryFiles(fixture)).toEqual([]);
    });
  });

  it("attempts every rollback and aggregates recovery failures behind the primary error", () => {
    withFixture((fixture) => {
      const first = path.join(fixture, "first.txt");
      const second = path.join(fixture, "second.txt");
      const third = path.join(fixture, "third.txt");
      fs.writeFileSync(first, "first original\n");
      fs.writeFileSync(second, "second original\n");
      fs.writeFileSync(third, "third original\n");

      const primaryError = new Error("injected third forward rename failure");
      const rollbackError = new Error("injected second rollback failure");
      const originalRename = fs.renameSync.bind(fs);
      let renameCalls = 0;
      vi.spyOn(fs, "renameSync").mockImplementation((source, destination) => {
        renameCalls += 1;
        if (renameCalls === 3) throw primaryError;
        if (renameCalls === 4) throw rollbackError;
        originalRename(source, destination);
      });

      let thrown: unknown;
      try {
        writeFilesAtomically([
          [relative(first), "first updated\n"],
          [relative(second), "second updated\n"],
          [relative(third), "third updated\n"],
        ]);
      } catch (error) {
        thrown = error;
      }

      expect(thrown).toBeInstanceOf(Error);
      const aggregate = thrown as Error & {
        cause?: unknown;
        errors: unknown[];
      };
      expect(aggregate.name).toBe("AggregateError");
      expect(aggregate.cause).toBe(primaryError);
      expect(aggregate.errors).toEqual([primaryError, rollbackError]);
      expect(aggregate.message).toContain(primaryError.message);
      expect(renameCalls).toBe(5);
      expect(fs.readFileSync(first, "utf8")).toBe("first original\n");
      expect(fs.readFileSync(second, "utf8")).toBe("second updated\n");
      expect(fs.readFileSync(third, "utf8")).toBe("third original\n");
      expect(temporaryFiles(fixture)).toEqual([]);
    });
  });
});
