import { execFileSync } from "node:child_process";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const GENERATOR = path.join(ROOT, "scripts", "tasks", "task-docs.mjs");
const DOCS_CHECKER = path.join(
  ROOT,
  "scripts",
  "tasks",
  "docs-contract-check.mjs",
);

type Generator = {
  escapeMarkdownCell(value: unknown): string;
  generateTaskDocs(): string;
};
type TaskDefinitions = Record<string, Record<string, unknown>>;
type DocsChecker = {
  validateMiseTaskReferences(
    file: string,
    source: string,
    tasks: TaskDefinitions,
  ): void;
  validateStandaloneSetup(file: string, source: string): void;
};

let generator: Generator;
let docsChecker: DocsChecker;

beforeAll(async () => {
  generator = (await import(
    /* @vite-ignore */ pathToFileURL(GENERATOR).href
  )) as Generator;
  docsChecker = (await import(
    /* @vite-ignore */ pathToFileURL(DOCS_CHECKER).href
  )) as DocsChecker;
});

describe("generated mise task documentation", () => {
  it("emits the generator banner without Trellis or Codex hook sections", () => {
    const document = generator.generateTaskDocs();
    expect(document).toContain(
      "> Generated from `.mise/tasks/*.toml` by `mise run tasks:docs:generate --apply`.",
    );
    expect(document).not.toContain("## Trellis and Codex Hooks");
    expect(document).not.toMatch(/\| `(?:trellis:|codex:hook)/);
  });

  it("documents every currently loaded task without freezing future extensions", () => {
    const tasks = JSON.parse(
      execFileSync("mise", ["tasks", "ls", "--local", "--json"], {
        cwd: ROOT,
        encoding: "utf8",
      }),
    ) as Array<{ name: string }>;
    const document = generator.generateTaskDocs();

    expect(tasks.length).toBeGreaterThanOrEqual(60);
    for (const task of tasks) {
      const escapedName = task.name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      expect(document, task.name).toMatch(
        new RegExp("\\| `" + escapedName + "` +\\|"),
      );
    }
  });

  it("escapes Markdown pipes and normalizes multiline metadata", () => {
    expect(generator.escapeMarkdownCell(String.raw`path\|next`)).toBe(
      String.raw`path\\\|next`,
    );
    expect(generator.escapeMarkdownCell("left|right\n next")).toBe(
      "left\\|right next",
    );
  });
});

describe("maintained mise reference contract", () => {
  const tasks: TaskDefinitions = {
    ["__proto__"]: {},
    a: {},
    bootstrap: {},
    constructor: {},
    "odd.task-1": {},
    "system:check": {},
  };

  it.each([
    ["lowercase short option", "mise run -s 'bash -lc' a"],
    ["boolean long options", "mise run --quiet --skip-tools --deny-net a"],
    ["boolean short cluster", "mise run -qf a"],
    [
      "equals and separate valued options",
      "mise run --jobs=2 --cd . -- odd.task-1",
    ],
    ["separate jobs value", "mise run --jobs 2 a"],
    ["known prototype-shaped own task", "mise run constructor"],
    ["known leading-underscore task", "mise run __proto__"],
    ["generic generated-doc placeholder", "Use `mise run <task>`."],
    [
      "backslash continuation",
      ["mise run --jobs \\", "  2 odd.task-1"].join("\n"),
    ],
    [
      "PowerShell backtick continuation",
      ["mise run --cd . `", "  -- a"].join("\n"),
    ],
    ["cmd caret continuation", ["mise run -q ^", "  a"].join("\n")],
  ])("parses and validates a mise run reference with %s", (_label, source) => {
    expect(() =>
      docsChecker.validateMiseTaskReferences("fixture.md", source, tasks),
    ).not.toThrow();
  });

  it.each([
    ["unknown task", "mise run missing", /unknown mise task/],
    [
      "inherited object key",
      "mise run toString",
      /unknown mise task: toString/,
    ],
    ["unknown option", "mise run --mystery a", /unknown mise run option/],
    [
      "value on boolean option",
      "mise run --quiet=true a",
      /invalid mise run option/,
    ],
    ["missing jobs value", "mise run --jobs", /missing mise run option value/],
    [
      "missing cd value before the option boundary",
      "mise run --cd -- a",
      /missing mise run option value/,
    ],
  ])(
    "fails closed for a mise reference with %s",
    (_label, source, expected) => {
      expect(() =>
        docsChecker.validateMiseTaskReferences("fixture.md", source, tasks),
      ).toThrow(expected);
    },
  );

  it("validates every mise run reference on the same line", () => {
    expect(() =>
      docsChecker.validateMiseTaskReferences(
        "fixture.md",
        "Use `mise run a` and then `mise run missing`.",
        tasks,
      ),
    ).toThrow(/unknown mise task: missing/);
  });

  it("does not impose a Trellis wrapper or legacy mise-exec policy", () => {
    expect(() =>
      docsChecker.validateMiseTaskReferences(
        "fixture.md",
        "Use `python -X utf8 .trellis/scripts/task.py list` for optional assistance. Legacy notes may mention `mise exec --`.",
        tasks,
      ),
    ).not.toThrow();
  });
});

describe("standalone setup contract", () => {
  const valid = [
    "After reviewing the configuration:",
    "```bash",
    "mise trust",
    "mise run bootstrap",
    "mise run system:check",
    "mise run dev",
    "```",
    "`mise trust` is a developer security decision and is never run automatically by a project task.",
    "Run `mise run check` before committing.",
  ].join("\n");

  it("keeps trust manual and the standalone setup sequence ordered", () => {
    expect(() =>
      docsChecker.validateStandaloneSetup("fixture.md", valid),
    ).not.toThrow();
  });

  it("rejects reordered setup commands", () => {
    expect(() =>
      docsChecker.validateStandaloneSetup(
        "fixture.md",
        valid.replace(
          "mise trust\nmise run bootstrap",
          "mise run bootstrap\nmise trust",
        ),
      ),
    ).toThrow(/exact order/);
  });

  it("rejects automatic trust ownership", () => {
    expect(() =>
      docsChecker.validateStandaloneSetup(
        "fixture.md",
        valid.replace(
          "is never run automatically by a project task",
          "is run automatically by a project task",
        ),
      ),
    ).toThrow(/manual developer decision/);
  });
});
