import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const require = createRequire(import.meta.url);
const read = (file: string) => fs.readFileSync(path.join(root, file), "utf8");
const configs = [
  "vite.config.ts",
  "vitest.config.ts",
  "playwright.config.ts",
  "playwright.performance.config.ts",
  "postcss.config.cjs",
  "dependency-cruiser.cjs",
];

describe("repository configuration ownership", () => {
  it("keeps only the automatic ESLint discovery entry among root code files", () => {
    const files = fs
      .readdirSync(root, { withFileTypes: true })
      .filter(
        (file) => file.isFile() && /\.(?:[cm]?[jt]s|tsx|jsx)$/.test(file.name),
      )
      .map((file) => file.name)
      .sort();
    expect(files).toEqual(["eslint.config.mjs"]);
    for (const name of configs) {
      expect(fs.existsSync(path.join(root, "config", name)), name).toBe(true);
      expect(fs.existsSync(path.join(root, name)), name).toBe(false);
    }
    expect(fs.existsSync(path.join(root, ".dependency-cruiser.cjs"))).toBe(
      false,
    );
  });

  it("uses actual built-in config discovery without root forwarding stubs", () => {
    // Vite/esbuild are Node tooling, not jsdom browser APIs. Execute the real
    // loader in its native realm rather than replacing Uint8Array/TextEncoder.
    const loaded = JSON.parse(
      execFileSync(
        process.execPath,
        [
          "--input-type=module",
          "-e",
          `
      import { loadConfigFromFile } from 'vite';
      const { config } = await loadConfigFromFile({command:'build', mode:'production'}, process.argv[1], process.cwd());
      console.log(JSON.stringify({root:config.root, envDir:config.envDir, alias:config.resolve.alias, postcss:config.css.postcss}));
    `,
          path.join(root, "config/vite.config.ts"),
        ],
        { cwd: root, encoding: "utf8" },
      ),
    ) as {
      root: string;
      envDir: string;
      alias: Record<string, string>;
      postcss: string;
    };
    expect(loaded.root).toBe(path.join(root, "src"));
    expect(loaded.envDir).toBe(path.join(root, "src"));
    expect(loaded.alias).toEqual({
      "@": path.join(root, "src"),
    });
    expect(loaded.postcss).toBe(path.join(root, "config") + path.sep);
    expect(require(path.join(root, "config/postcss.config.cjs"))).toEqual({
      plugins: { autoprefixer: {} },
    });
  });

  it("keeps every public runner on its one explicit configuration", () => {
    const manifest = JSON.parse(read("package.json")) as {
      scripts: Record<string, string>;
    };
    for (const name of [
      "test:unit",
      "test:unit:watch",
      "test:desktop:mock",
      "test:native-fetch",
    ]) {
      expect(manifest.scripts[name], name).toContain(
        "--config config/vitest.config.ts",
      );
    }
    expect(manifest.scripts["dev:renderer"]).toBe(
      "vite --config config/vite.config.ts",
    );
    expect(manifest.scripts["build:renderer"]).toContain(
      "vite build --config config/vite.config.ts",
    );
    expect(manifest.scripts["test:browser"]).toContain(
      "--config config/playwright.config.ts",
    );
    expect(manifest.scripts["test:performance"]).toContain(
      "--config config/playwright.performance.config.ts",
    );
    expect(manifest.scripts.lint).toContain(" config");
    expect(read("tsconfig.json")).toContain("config/**/*.ts");
    expect(read("eslint.config.mjs")).toContain("config/**/*.ts");
    expect(read("eslint.config.mjs")).toContain("config/**/*.cjs");
  });

  it("anchors both browser suites and their servers to the checkout, not config/", () => {
    for (const name of [
      "playwright.config.ts",
      "playwright.performance.config.ts",
    ]) {
      const source = read(`config/${name}`);
      expect(source).toContain('new URL("..", import.meta.url)');
      expect(source).toContain(
        'testDir: path.join(repositoryRoot, "tests/browser")',
      );
      expect(source).toContain("cwd: repositoryRoot");
      expect(source).toContain("reuseExistingServer: false");
    }
    expect(read("config/playwright.performance.config.ts")).toContain(
      "workers: 1",
    );
    expect(read("config/playwright.performance.config.ts")).toContain(
      "vite preview --config config/vite.config.ts",
    );
  });
});
