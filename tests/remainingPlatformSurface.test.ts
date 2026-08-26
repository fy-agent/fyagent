import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { deflateSync } from "node:zlib";
import { beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const CHECKER = path.join(
  ROOT,
  "scripts",
  "tasks",
  "supported-platform-check.mjs",
);

type Finding = {
  path: string;
  line: number;
  rule: string;
  excerpt: string;
};

type SurfaceMarkers = {
  kernel: string;
  subsystem: string;
  runnerFamily: string;
  distributions: readonly string[];
  imagePackage: string;
  sandboxPackage: string;
  sandboxCatalog: string;
  archivePackage: string;
  nativePackage: string;
  displayToolkit: string;
  embeddedToolkit: string;
  windowProtocol: string;
  compositorProtocol: string;
  directoryConvention: string;
  objectFormat: string;
  serviceManager: string;
  messageBus: string;
  packageCommands: readonly string[];
  broadRustFamily: string;
  displayEnvironment: string;
  packageAddCommand: string;
  sandboxInstallCommand: string;
};

type RustAllowance = {
  id: string;
  file: string;
  condition: string;
  next: string;
  nextPrefix?: boolean;
};

type SourceContract = { id: string; file: string; snippet: string };

type CheckerModule = {
  ACTIVE_TASK_ENV: string;
  DEVELOPMENT_HOST_ADMISSION_PATHS: readonly string[];
  GENERATED_STANDALONE_PREVIEW_PATH: string;
  MACOS_POSIX_CONTRACT: readonly SourceContract[];
  RUST_ALLOWANCE_CONTRACT: readonly RustAllowance[];
  RASTER_ASSET_CONTRACT: readonly { path: string; digest: string }[];
  STRUCTURE_ASSET_CONTRACT: readonly { path: string; digest: string }[];
  SURFACE_MARKERS: SurfaceMarkers;
  inspectRepository(options?: Record<string, unknown>): {
    findings: Finding[];
    inspectedFiles: number;
  };
  inspectKnownImage(relativePath: string, buffer: Buffer): string | undefined;
  loadRasterAssetManifest(
    manifestPath?: string,
    io?: unknown,
  ): readonly {
    path: string;
    digest: string;
  }[];
  loadStructureAssetManifest(
    manifestPath?: string,
    io?: unknown,
  ): readonly {
    path: string;
    digest: string;
  }[];
  isExcludedPath(relativePath: string, activeTask?: string): boolean;
  isTextExcludedPath(relativePath: string): boolean;
  listCurrentFiles(
    root?: string,
    runner?: (...args: unknown[]) => unknown,
  ): string[];
  listArchiveIndexModes(
    root?: string,
    runner?: (...args: unknown[]) => unknown,
  ): Map<string, string>;
  listCurrentIndexModes(
    root?: string,
    runner?: (...args: unknown[]) => unknown,
  ): Map<string, string>;
  parseArguments(
    argv: string[],
    environment?: Record<string, string>,
  ): string | undefined;
  readCurrentEntry(
    root: string,
    relativePath: string,
    io?: unknown,
    indexMode?: string,
  ): unknown;
  resolveAuthoritativeActiveTask(
    root?: string,
    runner?: (...args: unknown[]) => unknown,
  ): string;
  scanJavaScriptImplicitPredicates(
    entries: Array<{ path: string; source: string }>,
  ): Finding[];
  scanMacosPosixContract(
    entries: Array<{ path: string; source: string }>,
  ): Finding[];
  scanDirectoryConventionContract(
    entries: Array<{ path: string; source: string }>,
  ): Finding[];
  scanCargoImplicitPredicates(
    entries: Array<{ path: string; source: string }>,
  ): Finding[];
  scanPath(relativePath: string): Finding[];
  scanRustImplicitPredicates(
    entries: Array<{ path: string; source: string }>,
  ): Finding[];
  scanText(relativePath: string, source: string): Finding[];
  validateActiveTaskExclusion(
    value: string,
    options?: Record<string, unknown>,
  ): string;
  validateArchiveEntry(
    root: string,
    relativePath: string,
    io?: unknown,
    indexMode?: string,
  ): Finding[];
  validateRasterAssetInventory(
    currentPaths: string[],
    activeTask?: string,
  ): Finding[];
  validateStructureAssetInventory(
    currentPaths: string[],
    indexModes: Map<string, string>,
    options?: Record<string, unknown>,
  ): string[];
};

let checker: CheckerModule;

beforeAll(async () => {
  checker = (await import(
    /* @vite-ignore */ pathToFileURL(CHECKER).href
  )) as CheckerModule;
});

function activeTaskFixture(taskDirectoryName = "08-14-example-active-task") {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-surface-"));
  const relative = `.trellis/tasks/${taskDirectoryName}`;
  const directory = path.join(root, ...relative.split("/"));
  fs.mkdirSync(directory, { recursive: true });
  const id = taskDirectoryName.replace(/^\d+-\d+-/u, "");
  fs.writeFileSync(
    path.join(directory, "task.json"),
    `${JSON.stringify({ id, name: id, status: "in_progress" })}\n`,
  );
  return { directory, relative, root };
}

function permittedRustEntries() {
  const files: string[] = [];
  const visit = (relativeDirectory: string) => {
    const absoluteDirectory = path.join(ROOT, relativeDirectory);
    for (const entry of fs.readdirSync(absoluteDirectory, {
      withFileTypes: true,
    })) {
      const relativePath = path.posix.join(relativeDirectory, entry.name);
      if (entry.isDirectory()) visit(relativePath);
      if (entry.isFile() && relativePath.endsWith(".rs")) {
        files.push(relativePath);
      }
    }
  };
  visit("src-tauri/src");
  files.push("src-tauri/build.rs", "src-tauri/user-helper/build.rs");
  return files.map((relativePath) => ({
    path: relativePath,
    source: fs.readFileSync(path.join(ROOT, relativePath), "utf8"),
  }));
}

function macosPosixEntries() {
  return [...new Set(checker.MACOS_POSIX_CONTRACT.map(({ file }) => file))].map(
    (relativePath) => ({
      path: relativePath,
      source: fs.readFileSync(path.join(ROOT, relativePath), "utf8"),
    }),
  );
}

function pngCrc32(buffer: Buffer) {
  let crc = 0xffffffff;
  for (const value of buffer) {
    crc ^= value;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc & 1) !== 0 ? 0xedb88320 ^ (crc >>> 1) : crc >>> 1;
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type: string, payload: Buffer) {
  const typeBytes = Buffer.from(type, "ascii");
  const header = Buffer.alloc(8);
  header.writeUInt32BE(payload.length, 0);
  typeBytes.copy(header, 4);
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE(pngCrc32(Buffer.concat([typeBytes, payload])), 0);
  return Buffer.concat([header, payload, checksum]);
}

function pngWithMetadata(chunks: Buffer[]) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(1, 0);
  header.writeUInt32BE(1, 4);
  header[8] = 8;
  header[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pngChunk("IHDR", header),
    ...chunks,
    pngChunk("IDAT", Buffer.from([0])),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

describe("durable supported-platform surface contract", () => {
  it("constructs every retired marker without making the checker or test self-match", () => {
    for (const relativePath of [
      "scripts/tasks/supported-platform-check.mjs",
      "tests/remainingPlatformSurface.test.ts",
    ]) {
      const source = fs.readFileSync(path.join(ROOT, relativePath), "utf8");
      expect(checker.scanText(relativePath, source), relativePath).toEqual([]);
      expect(checker.scanPath(relativePath), relativePath).toEqual([]);
    }

    const admission = checker.DEVELOPMENT_HOST_ADMISSION_PATHS;
    expect([...admission]).toEqual(
      [...admission].sort((left, right) => left.localeCompare(right, "en")),
    );
    expect(new Set(admission).size).toBe(admission.length);
    const kernel = checker.SURFACE_MARKERS.kernel;
    expect(checker.scanText(admission[0], `${kernel}-x64`)).toEqual([]);
    expect(
      checker.scanText(admission[0], checker.SURFACE_MARKERS.imagePackage),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "image-package" }),
      ]),
    );
    expect(checker.scanText("src/lib/platform.ts", kernel)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "retired-kernel" }),
      ]),
    );
    const hostedRunner = `${checker.SURFACE_MARKERS.runnerFamily}-24.04`;
    expect(
      checker.scanText(
        ".github/workflows/star-history.yml",
        `    runs-on: ${hostedRunner}`,
      ),
    ).toEqual([]);
    expect(
      checker.scanText("src/notes.ts", checker.SURFACE_MARKERS.runnerFamily),
    ).toEqual([]);
  });

  it("keeps the always-run checker import closure on Node builtins only", () => {
    const source = fs.readFileSync(CHECKER, "utf8");
    const imports = Array.from(
      source.matchAll(/^import\s+[^;]+?\s+from\s+["']([^"']+)["'];?$/gmu),
      (match) => match[1],
    );
    expect(imports.length).toBeGreaterThan(0);
    expect(imports.every((specifier) => specifier.startsWith("node:"))).toBe(
      true,
    );
    expect(source).not.toContain('from "./lib.mjs"');
  });

  it("rejects directory-convention markers in source and asset paths", () => {
    const marker = checker.SURFACE_MARKERS.directoryConvention;
    for (const relativePath of [
      `src/${marker}-helper.ts`,
      `assets/${marker}-icon.png`,
    ]) {
      expect(checker.scanPath(relativePath)).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ rule: "path:directory-convention" }),
        ]),
      );
    }
  });

  it("rejects direct platform, distribution, package, display, and host-probe samples", () => {
    const markers = checker.SURFACE_MARKERS;
    const samples = [
      markers.kernel,
      `is${markers.kernel}`,
      `${markers.subsystem}.exe`,
      ...markers.distributions,
      markers.imagePackage,
      markers.sandboxPackage,
      markers.sandboxCatalog,
      `artifact.${markers.archivePackage}`,
      `artifact.${markers.nativePackage}`,
      markers.displayToolkit,
      markers.embeddedToolkit,
      markers.windowProtocol,
      markers.compositorProtocol,
      markers.objectFormat,
      markers.serviceManager,
      markers.messageBus,
      `"${markers.displayEnvironment}"`,
      ["", ["pr", "oc"].join(""), "version"].join("/"),
      ["", "etc", "os-release"].join("/"),
      ["", "mnt", "c", "tools"].join("/"),
      ["", ["ho", "me"].join(""), "demo"].join("/"),
      ["[Desktop", " Entry]"].join(""),
      ['{ "tar', 'gets": "all" }'].join(""),
      ["platform ", '!== "win32"'].join(""),
      ['"win32" !== ', "process.platform"].join(""),
      ["!is", "Windows()"].join(""),
      ...markers.packageCommands,
      `${markers.packageAddCommand} add package`,
      `${markers.sandboxInstallCommand} install package`,
    ];

    for (const [index, sample] of samples.entries()) {
      expect(
        checker.scanText(`fixture-${index}.txt`, sample),
        sample,
      ).not.toEqual([]);
    }
  });

  it("allows only the exact tested macOS POSIX contracts", () => {
    const terminology = `${checker.SURFACE_MARKERS.directoryConvention.toUpperCase()}_DATA_HOME`;
    expect(checker.scanText("notes/posix.txt", terminology)).toEqual([]);

    const entries = [
      ...macosPosixEntries(),
      {
        path: "tests/codexWindowsUserScopeContract.test.ts",
        source: fs.readFileSync(
          path.join(ROOT, "tests/codexWindowsUserScopeContract.test.ts"),
          "utf8",
        ),
      },
    ];
    expect(checker.scanMacosPosixContract(entries)).toEqual([]);
    const first = checker.MACOS_POSIX_CONTRACT[0];
    const drifted = entries.map((entry) =>
      entry.path === first.file
        ? { ...entry, source: entry.source.replace(first.snippet, "") }
        : entry,
    );
    expect(checker.scanMacosPosixContract(drifted)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "macos-posix:contract-drift" }),
      ]),
    );

    expect(checker.scanDirectoryConventionContract(entries)).toEqual([]);
    const unexpectedVariable = [
      checker.SURFACE_MARKERS.directoryConvention.toUpperCase(),
      "_CACHE_HOME",
    ].join("");
    expect(
      checker.scanDirectoryConventionContract([
        ...entries,
        { path: "src/cache.ts", source: `process.env.${unexpectedVariable}` },
      ]),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "macos-posix:unexpected-variable" }),
      ]),
    );
    const dataEntry = entries.find(
      ({ path: entryPath }) => entryPath === "src-tauri/src/opencode_config.rs",
    );
    const dataHomeIdentifier = ["OPENCODE_DATA_", "HOME_ENV"].join("");
    expect(dataEntry).toBeDefined();
    expect(
      checker.scanDirectoryConventionContract(
        entries.map((entry) =>
          entry === dataEntry
            ? {
                ...entry,
                source: `${entry.source}\n#[cfg(target_os = "windows")]\nfn leaked() { std::env::var_os(${dataHomeIdentifier}); }`,
              }
            : entry,
        ),
      ),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "macos-posix:unexpected-variable" }),
      ]),
    );

    const convention = checker.SURFACE_MARKERS.directoryConvention;
    const swapped = entries.map((entry) =>
      entry.path === "src-tauri/src/opencode_config.rs"
        ? {
            ...entry,
            source: `${entry.source.replace(
              `/// macOS 优先级: OPENCODE_DB 环境变量 > ${terminology} > ~/.local/share/opencode/opencode.db。`,
              "",
            )}\n#[cfg(target_os = "windows")]\nfn leaked() { std::env::var_os("${terminology}"); }`,
          }
        : entry,
    );
    expect(checker.scanDirectoryConventionContract(swapped)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "macos-posix:contract-drift" }),
        expect.objectContaining({ rule: "macos-posix:unexpected-variable" }),
      ]),
    );
    const cliRead = checker.MACOS_POSIX_CONTRACT.find(
      (contract) => contract.id === "cli-bin-macos-read",
    );
    expect(cliRead).toBeDefined();
    expect(
      checker.scanDirectoryConventionContract(
        entries.map((entry) =>
          entry.path === cliRead?.file
            ? {
                ...entry,
                source: entry.source.replace(
                  cliRead.snippet,
                  cliRead.snippet.replace(
                    'target_os = "macos"',
                    'target_os = "windows"',
                  ),
                ),
              }
            : entry,
        ),
      ),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "macos-posix:contract-drift" }),
      ]),
    );
    expect(
      checker.scanDirectoryConventionContract([
        ...entries,
        {
          path: "src/open-command.ts",
          source: [convention, "-open"].join(""),
        },
      ]),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "macos-posix:unexpected-variable" }),
      ]),
    );
  });

  it("audits filenames and strips only encoded SVG payload bytes", () => {
    const marker = checker.SURFACE_MARKERS.kernel;
    expect(checker.scanPath(`assets/${marker}-package.json`)).not.toEqual([]);

    const encoded = Buffer.from(marker, "utf8").toString("base64");
    const opaque = `<svg><image href="data:image/png;base64,${encoded}" /></svg>`;
    expect(checker.scanText("assets/icon.svg", opaque)).toEqual([]);
    expect(
      checker.scanText(
        "assets/icon.svg",
        `${opaque}\n<title>${marker}</title>`,
      ),
    ).not.toEqual([]);
  });

  it("keeps the exclusion set closed and the temporary task input exact", () => {
    const marker = checker.SURFACE_MARKERS.kernel;
    expect(
      checker.isExcludedPath(`.trellis/tasks/archive/${marker}/task.json`),
    ).toBe(true);
    expect(checker.isExcludedPath(`.trellis/tasks/current/${marker}.md`)).toBe(
      true,
    );
    expect(checker.isExcludedPath(".codebuddy/agents/example.md")).toBe(true);
    expect(checker.isExcludedPath(".codex/agents/example.md")).toBe(true);
    expect(checker.isExcludedPath(".agents/skills/example/SKILL.md")).toBe(
      true,
    );
    expect(checker.isTextExcludedPath("pnpm-lock.yaml")).toBe(true);
    expect(checker.isTextExcludedPath("src-tauri/Cargo.lock")).toBe(true);
    expect(checker.isTextExcludedPath("mise.lock")).toBe(false);

    const standalone = checker.GENERATED_STANDALONE_PREVIEW_PATH;
    expect(standalone).toBe("FyAgent-前端交互预览.html");
    expect(checker.listCurrentFiles(ROOT)).not.toContain(standalone);
    expect(checker.isExcludedPath(standalone)).toBe(false);
    expect(checker.isTextExcludedPath(standalone)).toBe(true);
    expect(checker.isTextExcludedPath(`nested/${standalone}`)).toBe(false);
    expect(checker.isTextExcludedPath("scripts/build-v2-preview.mjs")).toBe(
      false,
    );
    expect(checker.isTextExcludedPath("src/v2/main.tsx")).toBe(false);

    const fixture = activeTaskFixture();
    const directSession = () => fixture.relative;
    try {
      expect(
        checker.validateActiveTaskExclusion(fixture.relative, {
          root: fixture.root,
          sessionResolver: directSession,
        }),
      ).toBe(fixture.relative);
      expect(
        checker.isExcludedPath(
          `${fixture.relative}/research.md`,
          fixture.relative,
        ),
      ).toBe(true);
    } finally {
      fs.rmSync(fixture.root, { recursive: true, force: true });
    }
  });

  it("accepts any canonical task identity bound to the direct current session", () => {
    for (const taskName of [
      "01-02-first-valid-task",
      "12-31-second-valid-task",
      "08-14-agent-catalog-interactions",
    ]) {
      const fixture = activeTaskFixture(taskName);
      try {
        expect(
          checker.validateActiveTaskExclusion(fixture.relative, {
            root: fixture.root,
            sessionResolver: () => fixture.relative,
          }),
        ).toBe(fixture.relative);
      } finally {
        fs.rmSync(fixture.root, { recursive: true, force: true });
      }
    }
  });

  it("rejects noncanonical, nested, linked, mismatched, and completed tasks", () => {
    const fixture = activeTaskFixture();
    const directSession = () => fixture.relative;
    const metadataPath = path.join(fixture.directory, "task.json");
    const id = path.basename(fixture.directory).replace(/^\d+-\d+-/u, "");
    try {
      for (const invalid of [
        ".trellis/tasks/*",
        `${fixture.relative}/child`,
        `${fixture.relative}/../08-14-other-task`,
        fixture.relative.split("/").join("\\"),
        `.trellis/tasks/archive/${path.basename(fixture.relative)}`,
        ".trellis/tasks/00-14-invalid-month",
        ".trellis/tasks/08-32-invalid-day",
        ".trellis/tasks/08-14-Uppercase",
      ]) {
        expect(() =>
          checker.validateActiveTaskExclusion(invalid, {
            root: fixture.root,
            sessionResolver: directSession,
          }),
        ).toThrow();
      }

      const taskSymlinkIo = {
        ...fs,
        lstatSync(target: fs.PathLike) {
          if (
            path.resolve(String(target)) === path.resolve(fixture.directory)
          ) {
            return {
              isDirectory: () => true,
              isFile: () => false,
              isSymbolicLink: () => true,
            };
          }
          return fs.lstatSync(target);
        },
      };
      expect(() =>
        checker.validateActiveTaskExclusion(fixture.relative, {
          root: fixture.root,
          io: taskSymlinkIo,
          sessionResolver: directSession,
        }),
      ).toThrow(/real task directory/);

      const metadataSymlinkIo = {
        ...fs,
        lstatSync(target: fs.PathLike) {
          if (path.resolve(String(target)) === path.resolve(metadataPath)) {
            return {
              isDirectory: () => false,
              isFile: () => true,
              isSymbolicLink: () => true,
            };
          }
          return fs.lstatSync(target);
        },
      };
      expect(() =>
        checker.validateActiveTaskExclusion(fixture.relative, {
          root: fixture.root,
          io: metadataSymlinkIo,
          sessionResolver: directSession,
        }),
      ).toThrow(/regular task metadata/);

      for (const metadata of [
        { id, name: id, status: "complete" },
        { id: "other-task", name: id, status: "in_progress" },
        { id, name: "other-task", status: "in_progress" },
      ]) {
        fs.writeFileSync(metadataPath, `${JSON.stringify(metadata)}\n`);
        expect(() =>
          checker.validateActiveTaskExclusion(fixture.relative, {
            root: fixture.root,
            sessionResolver: directSession,
          }),
        ).toThrow(/metadata does not match/);
      }

      fs.writeFileSync(
        metadataPath,
        `${JSON.stringify({ id, name: id, status: "in_progress" })}\n`,
      );
      expect(() =>
        checker.validateActiveTaskExclusion(fixture.relative, {
          root: fixture.root,
          sessionResolver: () => ".trellis/tasks/08-14-other-task",
        }),
      ).toThrow(/does not match the current session task/);
    } finally {
      fs.rmSync(fixture.root, { recursive: true, force: true });
    }
  });

  it("accepts only one explicit argument channel", () => {
    const expected = ".trellis/tasks/08-14-example-active-task";
    expect(
      checker.parseArguments(["--exclude-active-task", expected], {}),
    ).toBe(expected);
    expect(
      checker.parseArguments([], { usage_exclude_active_task: expected }),
    ).toBe(expected);
    expect(
      checker.parseArguments([], { [checker.ACTIVE_TASK_ENV]: expected }),
    ).toBe(expected);
    expect(checker.parseArguments([], {})).toBeUndefined();
    expect(() =>
      checker.parseArguments(["--exclude-active-task", expected], {
        usage_exclude_active_task: expected,
      }),
    ).toThrow(/multiple inputs/);
    expect(() =>
      checker.parseArguments([], {
        usage_exclude_active_task: expected,
        [checker.ACTIVE_TASK_ENV]: expected,
      }),
    ).toThrow(/multiple inputs/);
    expect(() => checker.parseArguments(["--unknown"], {})).toThrow(/Usage/);
  });

  it("requires a direct current-session pointer for the active task", () => {
    const expected = ".trellis/tasks/08-14-example-active-task";
    const run =
      (payload: unknown, status = 0) =>
      () => ({
        error: undefined,
        status,
        stdout: JSON.stringify(payload),
      });
    expect(
      checker.resolveAuthoritativeActiveTask(
        ROOT,
        run({
          current_task: { dir: expected },
          source: "session:codex_expected",
          stale: false,
        }),
      ),
    ).toBe(expected);
    expect(() =>
      checker.resolveAuthoritativeActiveTask(
        ROOT,
        run({ current_task: null, source: "none", stale: false }, 1),
      ),
    ).toThrow(/no active-task pointer/);
    expect(() =>
      checker.resolveAuthoritativeActiveTask(
        ROOT,
        run({
          current_task: { dir: expected },
          source: "session-fallback:codex_other",
          stale: false,
        }),
      ),
    ).toThrow(/not directly active/);
    expect(() =>
      checker.resolveAuthoritativeActiveTask(
        ROOT,
        run({
          current_task: { dir: expected },
          source: "session:codex_expected",
          stale: true,
        }),
      ),
    ).toThrow(/not directly active/);
  });

  it("freezes every fail-closed Rust allowance by file, condition, and adjacent structure", () => {
    const entries = permittedRustEntries();
    expect(checker.RUST_ALLOWANCE_CONTRACT).toHaveLength(10);
    expect(checker.scanRustImplicitPredicates(entries)).toEqual([]);

    const first = checker.RUST_ALLOWANCE_CONTRACT[0];
    const drifted = entries.map((entry) =>
      entry.path === first.file
        ? {
            ...entry,
            source: entry.source.replace(first.next, `${first.next} drift`),
          }
        : entry,
    );
    expect(checker.scanRustImplicitPredicates(drifted)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "rust:allowance-drift" }),
        expect.objectContaining({ rule: "rust:implicit-target" }),
      ]),
    );

    const broad = {
      path: "src-tauri/tests/fixture.rs",
      source: `#[cfg(\n  ${checker.SURFACE_MARKERS.broadRustFamily}\n)]\nfn fixture() {}`,
    };
    expect(checker.scanRustImplicitPredicates([...entries, broad])).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          path: broad.path,
          rule: "rust:implicit-target",
        }),
      ]),
    );

    const invertedMac = {
      path: "src-tauri/tests/inverted.rs",
      source: '#[cfg(not(target_os = "macos"))]\nfn fallback() {}',
    };
    expect(
      checker.scanRustImplicitPredicates([...entries, invertedMac]),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          path: invertedMac.path,
          rule: "rust:implicit-target",
        }),
      ]),
    );

    for (const [name, source] of Object.entries({
      family: '#[cfg(not(target_family = "windows"))]\nfn fallback() {}',
      macro: 'fn fallback() { if cfg!(not(target_os = "windows")) {} }',
      multilineMacro:
        'fn fallback() { if cfg!(\n  not(target_os = "windows")\n) { generic(); } }',
      unary: 'fn fallback() { if !cfg!(target_os = "macos") {} }',
      positive:
        'fn fallback() { if cfg!(target_os = "windows") { windows() } else { generic() } }',
      nested:
        '#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]\nfn fallback() {}',
      thirdOs: `#[cfg(target_os = "${["hai", "ku"].join("")}")]\nfn fallback() {}`,
      broadFamily: `#[cfg(target_family = "${["w", "asm"].join("")}")]\nfn fallback() {}`,
      architecture: '#[cfg(target_arch = "x86_64")]\nfn fallback() {}',
      manual: `fn fallback() { let target_os = std::env::var("${[
        "CARGO_CFG_TARGET_",
        "OS",
      ].join("")}").unwrap(); if target_os != "windows" { generic(); } }`,
      runtimeOs: `fn fallback() { if std::env::consts::${["O", "S"].join(
        "",
      )} != "macos" { generic(); } }`,
    })) {
      expect(
        checker.scanRustImplicitPredicates([
          ...entries,
          { path: `src-tauri/tests/${name}.rs`, source },
        ]),
        name,
      ).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            path: `src-tauri/tests/${name}.rs`,
            rule:
              name === "manual" || name === "runtimeOs"
                ? "rust:manual-target"
                : "rust:implicit-target",
          }),
        ]),
      );
    }

    const movedMacro = entries.map((entry) =>
      entry.path === "src-tauri/src/lib.rs"
        ? {
            ...entry,
            source: `${entry.source.replace(
              'focus_main_window: cfg!(target_os = "macos"),',
              "focus_main_window: false,",
            )}\nfn generic_host_fallback() { let _ = cfg!(target_os = "macos"); }`,
          }
        : entry,
    );
    expect(checker.scanRustImplicitPredicates(movedMacro)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "rust:cfg-macro-drift" }),
      ]),
    );

    const movedArchitecture = entries.map((entry) =>
      entry.path === "src-tauri/src/codex_desktop_runtime.rs"
        ? {
            ...entry,
            source: `${entry.source.replace(
              '#[cfg(target_arch = "x86_64")]\n    {\n        CpuArchitecture::X86_64\n    }',
              "{ CpuArchitecture::X86_64 }",
            )}\n#[cfg(target_arch = "x86_64")]\nfn generic_architecture() {}`,
          }
        : entry,
    );
    expect(checker.scanRustImplicitPredicates(movedArchitecture)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          rule: "rust:architecture-contract-drift",
        }),
      ]),
    );

    expect(
      checker.scanRustImplicitPredicates([
        ...entries,
        {
          path: "src-tauri/tests/reordered.rs",
          source:
            '#[cfg(all(not(feature = "x"), target_os = "windows"))]\nfn supported_only() {}',
        },
      ]),
    ).toEqual([]);

    expect(
      checker.scanCargoImplicitPredicates([
        {
          path: "src-tauri/Cargo.toml",
          source:
            '[target.\'cfg(not(target_os = "windows"))\'.dependencies]\nexample = "1"',
        },
        {
          path: "src-tauri/Cargo.toml",
          source: `[target.'cfg(any(target_os = "windows", target_os = "${[
            "hai",
            "ku",
          ].join("")}"))'.dependencies]\nexample = "1"`,
        },
        {
          path: "src-tauri/Cargo.toml",
          source:
            '[target.\'cfg(all(target_os = "macos", target_vendor = "wide"))\'.dependencies]\nexample = "1"',
        },
        {
          path: "src-tauri/Cargo.toml",
          source:
            '[target.\'cfg(not(any(feature = "x", not(any(target_os = "windows", target_os = "macos")))))\'.dependencies]\nexample = "1"',
        },
        ...[
          'target_arch = "x86_64"',
          'target_env = "msvc"',
          'feature = "x"',
        ].map((predicate) => ({
          path: "src-tauri/Cargo.toml",
          source: `[target.'cfg(any(target_os = "windows", ${predicate}))'.dependencies]\nexample = "1"`,
        })),
      ]),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "cargo:implicit-target" }),
      ]),
    );

    expect(
      checker.scanCargoImplicitPredicates([
        {
          path: "src-tauri/Cargo.toml",
          source:
            '[target.\'cfg(all(target_os = "windows", target_arch = "x86_64"))\'.dependencies]\nexample = "1"',
        },
        {
          path: "src-tauri/Cargo.toml",
          source:
            '[target."cfg(target_os = \\"macos\\")".dependencies]\nexample = "1"',
        },
        {
          path: "src-tauri/Cargo.toml",
          source: "['target'.'cfg(windows)'.'dependencies']\nexample = \"1\"",
        },
        {
          path: "src-tauri/Cargo.toml",
          source: '["target"."cfg(macos)"."dependencies"]\nexample = "1"',
        },
      ]),
    ).toEqual([]);

    for (const source of [
      "['target'.'cfg(not(target_os = \"windows\"))'.dependencies]\nexample = \"1\"",
      '["tar\\u0067et"."cfg(not(target_os = \\"windows\\"))".dependencies]\nexample = "1"',
      '[target]\n\'cfg(not(target_os = "windows"))\'.dependencies.example = "1"',
      'target.\'cfg(not(target_os = "windows"))\'.dependencies.example = "1"',
      'target = { \'cfg(not(target_os = "windows"))\' = { dependencies = { example = "1" } } }',
    ]) {
      expect(
        checker.scanCargoImplicitPredicates([
          { path: "src-tauri/Cargo.toml", source },
        ]),
        source,
      ).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ rule: "cargo:implicit-target" }),
        ]),
      );
    }

    const bareTriple = [
      "x86_64",
      "unknown",
      checker.SURFACE_MARKERS.kernel,
      "gnu",
    ].join("-");
    for (const selector of [bareTriple, `'${bareTriple}'`]) {
      expect(
        checker.scanCargoImplicitPredicates([
          {
            path: "src-tauri/Cargo.toml",
            source: `[target.${selector}.dependencies]\nexample = "1"`,
          },
        ]),
        selector,
      ).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ rule: "cargo:implicit-target" }),
        ]),
      );
    }
  });

  it("rejects equivalent JavaScript generic-host fallbacks", () => {
    const entries = [
      {
        path: "src/fallback.ts",
        source:
          'if ("win32" === process.platform) { return windows(); }\nreturn generic();',
      },
      {
        path: "src/switch.ts",
        source:
          'switch (process.platform) {\ncase "win32": return windows();\ndefault: return generic();\n}',
      },
      {
        path: "src/full-chain.ts",
        source:
          'if (process.platform === "win32") { return windows(); } else if (process.platform === "darwin") { return mac(); } else { return generic(); }',
      },
      {
        path: "src/full-switch.ts",
        source:
          'switch (process.platform) {\ncase "win32": { return windows(); }\ncase "darwin": { return mac(); }\ndefault: { return generic(); }\n}',
      },
      {
        path: "src/ternary.ts",
        source:
          'const selected = process.platform === "win32" ? windows() : generic();',
      },
      {
        path: "src/template-ternary.ts",
        source:
          'const selected = `${process.platform === "win32" ? windows() : generic()}`;',
      },
      {
        path: "src/unrelated.ts",
        source:
          'if (process.platform === "darwin") { logMac(); }\nif (process.platform === "win32") { return windows(); }\nreturn generic();',
      },
      {
        path: "src/sequential.ts",
        source:
          'if (process.platform === "win32") { return windows(); }\nif (process.platform === "darwin") { return mac(); }\nreturn generic();',
      },
      {
        path: "src/unbraced.ts",
        source:
          'if (process.platform === "win32") return windows();\nreturn generic();',
      },
      {
        path: "src/switch-hidden-throw.ts",
        source:
          'switch (process.platform) {\ncase "win32": return windows();\ncase "darwin": return mac();\ndefault: { return generic(); }\n}\nthrow new Error("unrelated");',
      },
      {
        path: "src/helper-chain.ts",
        source:
          "if (isWindows()) return windows();\nelse if (isMac()) return mac();\nelse return generic();",
      },
      {
        path: "src/macos-only.ts",
        source: "if (isMac()) return mac();\nreturn generic();",
      },
      {
        path: "src/reversed-negative.mts",
        source:
          'if ("win32" !== process?.platform) return generic();\nreturn windows();',
      },
      {
        path: "src/bracket-platform.cts",
        source:
          'if (process?.["platform"] === "darwin") return mac();\nreturn generic();',
      },
      {
        path: "src/negative-helper.ts",
        source:
          "if (host?.isMacOS?.() === false) return generic();\nreturn mac();",
      },
      {
        path: "src/reversed-helper.ts",
        source: "if (false === isMac()) return generic();\nreturn mac();",
      },
      {
        path: "src/member-helper.ts",
        source:
          "if (host.isWindows(options)) return windows();\nreturn generic();",
      },
      {
        path: "src/optional-platform.ts",
        source:
          'if (process?.platform === "win32") return windows();\nreturn generic();',
      },
      {
        path: "src/null-or.ts",
        source:
          'if (process.platform === "win32") return windows();\nif (process.platform === "darwin") return mac();\nreturn null || generic();',
      },
      {
        path: "src/false-or-switch.ts",
        source:
          'switch (process.platform) {\ncase "win32": return windows();\ncase "darwin": return mac();\ndefault: return false || generic();\n}',
      },
      {
        path: "src/multiline-ternary.ts",
        source:
          'const selected = process.platform === "win32"\n  ? windows()\n  : generic();',
      },
      {
        path: "src/throw-then-generic.ts",
        source:
          'switch (process.platform) {\ncase "win32": return windows();\ncase "darwin": return mac();\ndefault: { throw new Error("unsupported"); generic(); }\n}',
      },
      ...Object.entries({
        blockComment: "/*😀😀*/ ",
        lineComment: "// 😀😀\n",
        quotedData: 'const data = "😀😀";\n',
        templateData: "const data = `😀😀`;\n",
      }).map(([name, prefix]) => ({
        path: `src/astral-${name}.ts`,
        source: `${prefix}if (process.platform === "win32") win(); else generic();`,
      })),
    ];
    expect(checker.scanJavaScriptImplicitPredicates(entries)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          path: "src/fallback.ts",
          rule: "js:implicit-target",
        }),
        expect.objectContaining({
          path: "src/switch.ts",
          rule: "js:implicit-target",
        }),
        expect.objectContaining({ path: "src/full-chain.ts" }),
        expect.objectContaining({ path: "src/full-switch.ts" }),
        expect.objectContaining({ path: "src/ternary.ts" }),
        expect.objectContaining({ path: "src/template-ternary.ts" }),
        expect.objectContaining({ path: "src/unrelated.ts" }),
        expect.objectContaining({ path: "src/sequential.ts" }),
        expect.objectContaining({ path: "src/unbraced.ts" }),
        expect.objectContaining({ path: "src/switch-hidden-throw.ts" }),
        expect.objectContaining({ path: "src/helper-chain.ts" }),
        expect.objectContaining({ path: "src/macos-only.ts" }),
        expect.objectContaining({ path: "src/reversed-negative.mts" }),
        expect.objectContaining({ path: "src/bracket-platform.cts" }),
        expect.objectContaining({ path: "src/negative-helper.ts" }),
        expect.objectContaining({ path: "src/reversed-helper.ts" }),
        expect.objectContaining({ path: "src/member-helper.ts" }),
        expect.objectContaining({ path: "src/optional-platform.ts" }),
        expect.objectContaining({ path: "src/null-or.ts" }),
        expect.objectContaining({ path: "src/false-or-switch.ts" }),
        expect.objectContaining({ path: "src/multiline-ternary.ts" }),
        expect.objectContaining({ path: "src/throw-then-generic.ts" }),
        expect.objectContaining({ path: "src/astral-blockComment.ts" }),
        expect.objectContaining({ path: "src/astral-lineComment.ts" }),
        expect.objectContaining({ path: "src/astral-quotedData.ts" }),
        expect.objectContaining({ path: "src/astral-templateData.ts" }),
      ]),
    );

    expect(
      checker.scanJavaScriptImplicitPredicates([
        {
          path: "src/closed.ts",
          source:
            'if (process.platform === "win32") return windows();\nif (process.platform === "darwin") return mac();\nthrow new Error("unsupported");',
        },
      ]),
    ).toEqual([]);

    expect(
      checker.scanJavaScriptImplicitPredicates([
        {
          path: "src/fixture-data.ts",
          source:
            'const fixture = `if (process.platform === "win32") return generic();`;',
        },
        {
          path: "src/template-closed.ts",
          source:
            'const value = `${process.platform === "win32" ? windows() : null}`;',
        },
      ]),
    ).toEqual([]);
  });

  it("runs every production scanner against the current repository snapshot without lifecycle exclusions", () => {
    const indexModes = checker.listCurrentIndexModes(ROOT);
    const runner = (
      _command: unknown,
      arguments_: unknown,
      _options: unknown,
    ) => {
      if (
        Array.isArray(arguments_) &&
        arguments_.includes("--stage") &&
        !arguments_.includes(".trellis/tasks/archive/")
      ) {
        return {
          status: 0,
          stdout: Buffer.from(
            Array.from(
              indexModes,
              ([relativePath, mode]) =>
                `${mode} 0123456789abcdef0123456789abcdef01234567 0\t${relativePath}\0`,
            ).join(""),
          ),
        };
      }
      return checker.listCurrentFiles(ROOT).length > 0
        ? {
            status: 0,
            stdout: Buffer.from(
              `${checker.listCurrentFiles(ROOT).join("\0")}\0`,
            ),
          }
        : { status: 1, stdout: Buffer.alloc(0) };
    };
    const report = checker.inspectRepository({
      root: ROOT,
      runner,
      sessionResolver: () => {
        throw new Error("The archived snapshot must not query task authority");
      },
    });
    expect(report.findings).toEqual([]);
    expect(report.inspectedFiles).toBeGreaterThan(1_000);
  });

  it("fails closed when Git enumeration or file reads fail", () => {
    expect(() =>
      checker.listCurrentFiles(ROOT, () => ({
        error: undefined,
        status: 1,
        stdout: Buffer.alloc(0),
      })),
    ).toThrow(/Unable to enumerate/);

    const denied = Object.assign(new Error("denied"), { code: "EACCES" });
    expect(() =>
      checker.readCurrentEntry(ROOT, "README.md", {
        lstatSync() {
          throw denied;
        },
      }),
    ).toThrow("denied");

    const fixture = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-utf8-"));
    try {
      fs.writeFileSync(
        path.join(fixture, "invalid.txt"),
        Buffer.from([0xc3, 0x28]),
      );
      expect(() => checker.readCurrentEntry(fixture, "invalid.txt")).toThrow();

      const script = `Write-Output "${checker.SURFACE_MARKERS.kernel}"`;
      fs.writeFileSync(
        path.join(fixture, "encoded.ps1"),
        Buffer.concat([
          Buffer.from([0xff, 0xfe]),
          Buffer.from(script, "utf16le"),
        ]),
      );
      const entry = checker.readCurrentEntry(fixture, "encoded.ps1") as {
        source: string;
      };
      expect(checker.scanText("encoded.ps1", entry.source)).not.toEqual([]);

      fs.writeFileSync(
        path.join(fixture, "unknown.bin"),
        Buffer.from([0, 1, 2, 3]),
      );
      expect(() => checker.readCurrentEntry(fixture, "unknown.bin")).toThrow(
        /NUL-containing/,
      );

      const reviewedImage = "src-tauri/icons/32x32.png";
      const validPng = fs.readFileSync(path.join(ROOT, reviewedImage));
      fs.mkdirSync(path.dirname(path.join(fixture, reviewedImage)), {
        recursive: true,
      });
      fs.writeFileSync(path.join(fixture, reviewedImage), validPng);
      expect(
        checker.readCurrentEntry(fixture, reviewedImage, fs, "100644"),
      ).toEqual({
        path: reviewedImage,
        source: undefined,
      });

      fs.writeFileSync(
        path.join(fixture, reviewedImage),
        Buffer.concat([
          validPng,
          Buffer.from(checker.SURFACE_MARKERS.kernel, "utf8"),
        ]),
      );
      expect(() =>
        checker.readCurrentEntry(fixture, reviewedImage, fs, "100644"),
      ).toThrow(/identity is not reviewed/iu);

      fs.writeFileSync(
        path.join(fixture, reviewedImage),
        Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      );
      expect(() =>
        checker.readCurrentEntry(fixture, reviewedImage, fs, "100644"),
      ).toThrow(/identity is not reviewed/iu);
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("freezes the decoded and visually reviewed raster inventory by path and digest", () => {
    const currentPaths = checker.listCurrentFiles(ROOT);
    expect(checker.RASTER_ASSET_CONTRACT).toHaveLength(148);
    expect(checker.validateRasterAssetInventory(currentPaths)).toEqual([]);

    const first = checker.RASTER_ASSET_CONTRACT[0];
    expect(
      checker.validateRasterAssetInventory(
        currentPaths.filter((candidate) => candidate !== first.path),
      ),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "raster:inventory-drift" }),
      ]),
    );
    expect(
      checker.validateRasterAssetInventory([
        ...currentPaths,
        "assets/unreviewed.png",
      ]),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ rule: "raster:inventory-drift" }),
      ]),
    );
  });

  it("seals every platform-sensitive source by path, mode, and digest", () => {
    const currentPaths = checker.listCurrentFiles(ROOT);
    const indexModes = checker.listCurrentIndexModes(ROOT);
    for (const manifestPath of [
      "scripts/tasks/supported-platform-raster-assets.json",
      "scripts/tasks/supported-platform-structure-assets.json",
    ]) {
      indexModes.set(manifestPath, "100644");
    }
    const validate = (
      relativePath?: string,
      mutate?: (source: string) => string,
    ) => {
      const io = {
        lstatSync: fs.lstatSync,
        readFileSync(absolutePath: fs.PathOrFileDescriptor, options?: unknown) {
          const buffer = fs.readFileSync(absolutePath);
          const relative = path
            .relative(ROOT, String(absolutePath))
            .split(path.sep)
            .join("/");
          const result =
            relativePath === relative && mutate
              ? Buffer.from(mutate(buffer.toString("utf8")), "utf8")
              : buffer;
          return typeof options === "string"
            ? result.toString(options as BufferEncoding)
            : result;
        },
      };
      return checker.validateStructureAssetInventory(currentPaths, indexModes, {
        root: ROOT,
        io,
      });
    };

    expect(checker.STRUCTURE_ASSET_CONTRACT.length).toBeGreaterThan(50);
    expect(validate()).toEqual(
      checker.STRUCTURE_ASSET_CONTRACT.map(({ path: assetPath }) => assetPath),
    );

    const append = (snippet: string) => (source: string) =>
      `${source}\n${snippet}\n`;
    const mutations: Array<[string, (source: string) => string]> = [
      [
        "src-tauri/Cargo.toml",
        append("['target'.'cfg(not(target_os = \"windows\"))'.dependencies]"),
      ],
      [
        "src-tauri/Cargo.toml",
        append(
          'target.\'cfg(any(target_os = "windows", target_arch = "x86_64"))\'.dependencies.example = "1"',
        ),
      ],
      [
        "src-tauri/src/lib.rs",
        append(
          '#[cfg(any(target_os = "windows", feature = "portable"))]\nfn generic_target() {}',
        ),
      ],
      [
        "src-tauri/src/services/tooling.rs",
        (source) =>
          source.replace(
            '#[cfg(target_os = "macos")]\n        let ambient_paths',
            '#[cfg(target_os = "windows")]\n        let ambient_paths',
          ),
      ],
      ...["TARGET", "HOST", "FAMILY"].map(
        (selector): [string, (source: string) => string] => [
          "src-tauri/build.rs",
          append(
            `fn generic_${selector.toLowerCase()}() { let _ = std::env::var("${selector}"); }`,
          ),
        ],
      ),
      ...[
        'if ((process.platform) === "win32") windows(); else generic();',
        "if (isMac() !== true) generic();",
        'if (process.platform === "win32") windows(); else throw generic();',
      ].map((snippet): [string, (source: string) => string] => [
        "src/lib/platform.ts",
        append(snippet),
      ]),
    ];
    for (const [relativePath, mutate] of mutations) {
      expect(() => validate(relativePath, mutate), relativePath).toThrow(
        /structure identity drifted/iu,
      );
    }

    const first = checker.STRUCTURE_ASSET_CONTRACT[0].path;
    const wrongModes = new Map(indexModes);
    wrongModes.set(first, "100755");
    expect(() =>
      checker.validateStructureAssetInventory(currentPaths, wrongModes, {
        root: ROOT,
      }),
    ).toThrow(/mode 100644/iu);

    const added = "src/new-platform-probe.ts";
    const addedAbsolute = path.join(ROOT, ...added.split("/"));
    const addedModes = new Map(indexModes).set(added, "100644");
    expect(() =>
      checker.validateStructureAssetInventory(
        [...currentPaths, added].sort((left, right) =>
          left.localeCompare(right, "en"),
        ),
        addedModes,
        {
          root: ROOT,
          io: {
            lstatSync(absolutePath: fs.PathLike) {
              return absolutePath === addedAbsolute
                ? { isFile: () => true, isSymbolicLink: () => false }
                : fs.lstatSync(absolutePath);
            },
            readFileSync(absolutePath: fs.PathOrFileDescriptor) {
              return absolutePath === addedAbsolute
                ? Buffer.from('process.platform === "win32"', "utf8")
                : fs.readFileSync(absolutePath);
            },
          },
        },
      ),
    ).toThrow(/candidate inventory drifted/iu);
  });

  it("loads only an exact regular structure manifest", () => {
    const fixture = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-structure-"),
    );
    const manifestPath = path.join(fixture, "structure.json");
    try {
      const current = JSON.parse(
        fs.readFileSync(
          path.join(
            ROOT,
            "scripts",
            "tasks",
            "supported-platform-structure-assets.json",
          ),
          "utf8",
        ),
      );
      fs.writeFileSync(manifestPath, JSON.stringify(current));
      expect(checker.loadStructureAssetManifest(manifestPath)).toHaveLength(
        checker.STRUCTURE_ASSET_CONTRACT.length,
      );
      fs.writeFileSync(
        manifestPath,
        JSON.stringify({ ...current, unexpected: true }),
      );
      expect(() => checker.loadStructureAssetManifest(manifestPath)).toThrow(
        /manifest schema/iu,
      );
      expect(() =>
        checker.loadStructureAssetManifest(manifestPath, {
          lstatSync() {
            return { isFile: () => true, isSymbolicLink: () => true };
          },
          readFileSync() {
            throw new Error("structure manifest symlink was read");
          },
        }),
      ).toThrow(/regular non-symlink/iu);
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("loads only an exact regular raster manifest", () => {
    const fixture = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-manifest-"));
    const manifestPath = path.join(fixture, "raster.json");
    try {
      const current = JSON.parse(
        fs.readFileSync(
          path.join(
            ROOT,
            "scripts",
            "tasks",
            "supported-platform-raster-assets.json",
          ),
          "utf8",
        ),
      );
      fs.writeFileSync(manifestPath, JSON.stringify(current));
      expect(checker.loadRasterAssetManifest(manifestPath)).toHaveLength(148);

      fs.writeFileSync(
        manifestPath,
        JSON.stringify({ ...current, unexpected: true }),
      );
      expect(() => checker.loadRasterAssetManifest(manifestPath)).toThrow(
        /manifest schema/iu,
      );

      expect(() =>
        checker.loadRasterAssetManifest(manifestPath, {
          lstatSync() {
            return { isFile: () => true, isSymbolicLink: () => true };
          },
          readFileSync() {
            throw new Error("manifest symlink was read");
          },
        }),
      ).toThrow(/regular non-symlink/iu);
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("rejects raster symlinks and every non-regular Git index mode", () => {
    const fixture = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-mode-"));
    const reviewedImage = "src-tauri/icons/32x32.png";
    try {
      fs.mkdirSync(path.dirname(path.join(fixture, reviewedImage)), {
        recursive: true,
      });
      fs.copyFileSync(
        path.join(ROOT, reviewedImage),
        path.join(fixture, reviewedImage),
      );
      for (const mode of [undefined, "100755", "120000"]) {
        expect(() =>
          checker.readCurrentEntry(fixture, reviewedImage, fs, mode),
        ).toThrow(/regular 100644 Git file/iu);
      }
      expect(() =>
        checker.readCurrentEntry(
          fixture,
          reviewedImage,
          {
            lstatSync() {
              return { isFile: () => true, isSymbolicLink: () => true };
            },
            readFileSync() {
              throw new Error("raster symlink was read");
            },
          },
          "100644",
        ),
      ).toThrow(/regular 100644 Git file/iu);
    } finally {
      fs.rmSync(fixture, { recursive: true, force: true });
    }
  });

  it("caps aggregate raster metadata before decoding or inflation", () => {
    const compressed = deflateSync(Buffer.alloc(400_000, 0x41));
    const metadata = Buffer.concat([
      Buffer.from("key\0\0", "latin1"),
      compressed,
    ]);
    const aggregate = pngWithMetadata([
      pngChunk("zTXt", metadata),
      pngChunk("zTXt", metadata),
      pngChunk("zTXt", metadata),
    ]);
    expect(() => checker.inspectKnownImage("aggregate.png", aggregate)).toThrow(
      /metadata budget/iu,
    );

    const oversized = pngWithMetadata([
      pngChunk("tEXt", Buffer.alloc(1024 * 1024 + 1, 0x41)),
    ]);
    expect(() => checker.inspectKnownImage("oversized.png", oversized)).toThrow(
      /metadata budget/iu,
    );

    for (const [flag, method] of [
      [2, 0],
      [0, 1],
    ]) {
      const invalidInternationalText = pngWithMetadata([
        pngChunk(
          "iTXt",
          Buffer.concat([
            Buffer.from("key\0", "latin1"),
            Buffer.from([flag, method]),
            Buffer.from("\0\0text", "utf8"),
          ]),
        ),
      ]);
      expect(() =>
        checker.inspectKnownImage("invalid-itxt.png", invalidInternationalText),
      ).toThrow(/invalid PNG iTXt metadata/iu);
    }

    const emptyIcns = Buffer.alloc(8);
    emptyIcns.write("icns", 0, "ascii");
    emptyIcns.writeUInt32BE(8, 4);
    expect(() => checker.inspectKnownImage("empty.icns", emptyIcns)).toThrow(
      /no recognized image payload/iu,
    );
  });

  it("keeps archive exclusions structural and rejects executable or manifest payloads", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-archive-"));
    const historicalTask = ["08-12-remove", "lin", "ux-support"].join("-");
    const base = `.trellis/tasks/archive/2026-08/${historicalTask}`;
    try {
      const document = `${base}/prd.md`;
      fs.mkdirSync(path.dirname(path.join(root, document)), {
        recursive: true,
      });
      fs.writeFileSync(
        path.join(root, document),
        checker.SURFACE_MARKERS.kernel,
      );
      expect(
        checker.validateArchiveEntry(root, document, fs, "100644"),
      ).toEqual([]);
      expect(() =>
        checker.validateArchiveEntry(root, document, fs, "100755"),
      ).toThrow(/Git index mode/);
      expect(() =>
        checker.validateArchiveEntry(root, document, fs, "120000"),
      ).toThrow(/Git index mode/);
      const stagedMode = (mode: string) =>
        checker
          .listArchiveIndexModes(root, () => ({
            status: 0,
            stdout: Buffer.from(
              `${mode} 0123456789abcdef0123456789abcdef01234567 0\t${document}\0`,
            ),
          }))
          .get(document);
      expect(stagedMode("100644")).toBe("100644");
      for (const mode of ["100755", "120000"]) {
        expect(() =>
          checker.validateArchiveEntry(root, document, fs, stagedMode(mode)),
        ).toThrow(/Git index mode/);
      }

      const manifest = `${base}/package.json`;
      fs.writeFileSync(path.join(root, manifest), "{}");
      expect(() => checker.validateArchiveEntry(root, manifest)).toThrow(
        /standard task document/,
      );
      const researchJson = `${base}/research/evidence.json`;
      fs.mkdirSync(path.dirname(path.join(root, researchJson)), {
        recursive: true,
      });
      fs.writeFileSync(path.join(root, researchJson), '{"reviewed":true}\n');
      expect(
        checker.validateArchiveEntry(root, researchJson, fs, "100644"),
      ).toEqual([]);
      fs.writeFileSync(path.join(root, researchJson), "{");
      expect(() =>
        checker.validateArchiveEntry(root, researchJson, fs, "100644"),
      ).toThrow(SyntaxError);

      expect(() =>
        checker.validateArchiveEntry(
          root,
          document,
          {
            lstatSync() {
              return {
                isFile: () => true,
                isSymbolicLink: () => false,
                mode: 0o100755,
              };
            },
          },
          "100644",
        ),
      ).toThrow(/executable/);
      expect(() =>
        checker.validateArchiveEntry(
          root,
          document,
          {
            lstatSync() {
              return {
                isFile: () => true,
                isSymbolicLink: () => true,
                mode: 0o100644,
              };
            },
          },
          "100644",
        ),
      ).toThrow(/regular file/);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("publishes a read-only parameterized task inside the contract gate", async () => {
    const taskContract = (await import(
      /* @vite-ignore */ pathToFileURL(
        path.join(ROOT, "scripts", "tasks", "task-contract-check.mjs"),
      ).href
    )) as {
      loadTaskDefinitions(): Record<
        string,
        {
          env: { FYAGENT_TASK_EFFECT: string };
          run: unknown;
          usage?: string;
        }
      >;
    };
    const tasks = taskContract.loadTaskDefinitions();
    const task = tasks["supported-platform:check"];
    expect(task.env.FYAGENT_TASK_EFFECT).toBe("read-only");
    expect(task.usage).toContain("--exclude-active-task <path>");
    expect(tasks["check:contracts"].run).toEqual(
      expect.arrayContaining([{ task: "supported-platform:check" }]),
    );
  });
});
