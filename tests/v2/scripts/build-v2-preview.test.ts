import {
  mkdir,
  mkdtemp,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

// @ts-expect-error The standalone builder intentionally remains runtime JavaScript.
import * as previewBuilder from "../../../scripts/build-v2-preview.mjs";

const {
  buildV2Preview,
  parseDistributionEntryAssets,
  resolveDistributionAsset,
} = previewBuilder;

const temporaryRoots: string[] = [];

async function createDistributionFixture(
  indexHtml: string,
  assets: Record<string, string>,
) {
  const root = await mkdtemp(path.join(tmpdir(), "fyagent-v2-preview-"));
  temporaryRoots.push(root);
  const distributionDirectory = path.join(root, "dist");
  const assetsDirectory = path.join(distributionDirectory, "assets");
  const outputPath = path.join(root, "preview.html");

  await mkdir(assetsDirectory, { recursive: true });
  await writeFile(path.join(distributionDirectory, "index.html"), indexHtml);
  await Promise.all(
    Object.entries(assets).map(([name, content]) =>
      writeFile(path.join(assetsDirectory, name), content),
    ),
  );

  return { distributionDirectory, outputPath };
}

afterEach(async () => {
  await Promise.all(
    temporaryRoots
      .splice(0)
      .map((root) => rm(root, { force: true, recursive: true })),
  );
});

describe("V2 standalone preview builder", () => {
  it("fails fast when dist/index.html has no direct module entry", async () => {
    expect(() =>
      parseDistributionEntryAssets(
        "<!doctype html><html><head></head><body></body></html>",
      ),
    ).toThrow(/direct module entry script/i);

    const fixture = await createDistributionFixture(
      "<!doctype html><html><head></head><body></body></html>",
      {},
    );
    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /direct module entry script/i,
    );
  });

  it("removes whitespace-only indentation lines from the generated HTML", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html>
<html><head>
  ${"  "}
  <script type="module" src="./assets/entry.js"></script>
</head><body><div id="root"></div></body></html>`,
      { "entry.js": "window.entryLoaded = true;" },
    );

    await buildV2Preview(fixture);

    expect(await readFile(fixture.outputPath, "utf8")).not.toMatch(
      /^[\t ]+$/m,
    );
  });

  it("inlines every direct module script and stylesheet in HTML order", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html>
<html><head>
  <link rel="stylesheet" href="./assets/first.css" />
  <script type="module" src="./assets/first.js"></script>
  <link href='./assets/second.css' rel='stylesheet' />
</head><body><div id="root"></div>
  <script src='./assets/second.js' type='module'></script>
</body></html>`,
      {
        "first.css": "/* first-style */",
        "first.js": "window.firstEntry = true; // first-entry",
        "second.css": "/* second-style */",
        "second.js": "window.secondEntry = true; // second-entry",
      },
    );

    const distributionFiles = [
      "index.html",
      "assets/first.css",
      "assets/first.js",
      "assets/second.css",
      "assets/second.js",
    ];
    const distributionBefore = new Map(
      await Promise.all(
        distributionFiles.map(async (fileName) => [
          fileName,
          await readFile(path.join(fixture.distributionDirectory, fileName)),
        ] as const),
      ),
    );
    const result = await buildV2Preview(fixture);
    const standalone = await readFile(fixture.outputPath, "utf8");

    expect(result).toMatchObject({
      scriptEntryCount: 2,
      stylesheetEntryCount: 2,
    });
    for (const marker of [
      "first-style",
      "first-entry",
      "second-style",
      "second-entry",
    ]) {
      expect(standalone).toContain(marker);
    }
    expect(standalone.indexOf("first-style")).toBeLessThan(
      standalone.indexOf("first-entry"),
    );
    expect(standalone.indexOf("first-entry")).toBeLessThan(
      standalone.indexOf("second-style"),
    );
    expect(standalone.indexOf("second-style")).toBeLessThan(
      standalone.indexOf("second-entry"),
    );
    expect(standalone).not.toMatch(/<script\b[^>]*\bsrc\s*=/i);
    expect(standalone).not.toMatch(
      /<link\b(?=[^>]*\brel=["']stylesheet["'])(?=[^>]*\bhref=)[^>]*>/i,
    );
    for (const source of [
      "./assets/first.css",
      "./assets/first.js",
      "./assets/second.css",
      "./assets/second.js",
    ]) {
      expect(standalone).not.toContain(source);
    }
    for (const fileName of distributionFiles) {
      expect(
        await readFile(path.join(fixture.distributionDirectory, fileName)),
      ).toEqual(distributionBefore.get(fileName));
    }
  });

  it("supports zero stylesheets without treating arbitrary asset strings as modules", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script>
    if (window.location.protocol === "file:") {
      window.location.replace("../FyAgent-前端交互预览.html");
    }
  </script>
  <script type="module" src="./assets/entry.js"></script>
</head><body><div id="root"></div></body></html>`,
      {
        "entry.js":
          'const labels = ["large.js", "theme.css"]; window.currentGraphLoaded = new URL("./logo.png", import.meta.url).href;',
        "logo.png": "fixture-image",
      },
    );

    const indexPath = path.join(fixture.distributionDirectory, "index.html");
    const indexBefore = await readFile(indexPath);
    const result = await buildV2Preview(fixture);
    const standalone = await readFile(fixture.outputPath, "utf8");

    expect(result.stylesheetEntryCount).toBe(0);
    expect(standalone).toContain("window.__FYAGENT_STANDALONE_PREVIEW__ = true");
    expect(standalone).toContain("window.currentGraphLoaded");
    expect(standalone).toContain('"large.js"');
    expect(standalone).toContain('"theme.css"');
    expect(standalone).toContain("data:image/png;base64,");
    expect(standalone).not.toContain("window.location.replace");
    expect(standalone).not.toContain("./assets/entry.js");
    expect(await readFile(indexPath)).toEqual(indexBefore);
  });

  it("inlines the known Vite bootstrap as CSS then its mapped JS entry", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script>
    if (window.location.protocol === "file:") {
      window.location.replace("../FyAgent-前端交互预览.html");
    }
  </script>
  <script type="module">
    const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["./assets/main.js","./assets/main.css"])))=>i.map(i=>d[i]);
    const preload=(loader)=>loader();
    window.location.protocol!=="file:"&&preload(()=>import("./assets/main.js"),__vite__mapDeps([0,1]),import.meta.url);
  </script>
</head><body><div id="root"></div></body></html>`,
      {
        "main.js":
          'window.viteMainMarker = new URL("./logo.png", import.meta.url).href;',
        "main.css":
          '.vite-css-marker { background-image: url("./logo.png"); }',
        "logo.png": "vite-fixture-image",
      },
    );
    const distributionFiles = [
      "index.html",
      "assets/main.js",
      "assets/main.css",
      "assets/logo.png",
    ];
    const distributionBefore = new Map(
      await Promise.all(
        distributionFiles.map(async (fileName) => [
          fileName,
          await readFile(path.join(fixture.distributionDirectory, fileName)),
        ] as const),
      ),
    );

    const result = await buildV2Preview(fixture);
    const standalone = await readFile(fixture.outputPath, "utf8");

    expect(result).toMatchObject({
      scriptEntryCount: 1,
      stylesheetEntryCount: 1,
    });
    expect(standalone).toContain("window.__FYAGENT_STANDALONE_PREVIEW__ = true");
    expect(standalone).toContain("vite-css-marker");
    expect(standalone).toContain("viteMainMarker");
    expect(standalone).toContain("data:image/png;base64,");
    expect(standalone.indexOf("vite-css-marker")).toBeLessThan(
      standalone.indexOf("viteMainMarker"),
    );
    expect(standalone).not.toContain("__vite__mapDeps");
    expect(standalone).not.toContain("./assets/main.js");
    expect(standalone).not.toContain("./assets/main.css");
    expect(standalone).not.toContain("./logo.png");
    expect(standalone).not.toMatch(/<script\b[^>]*\bsrc\s*=/i);
    for (const fileName of distributionFiles) {
      expect(
        await readFile(path.join(fixture.distributionDirectory, fileName)),
      ).toEqual(distributionBefore.get(fileName));
    }
  });

  it("inlines the known external Vite bootstrap as CSS then its single leaf module", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script type="module" src="./assets/index.js"></script>
</head><body><div id="root"></div></body></html>`,
      {
        "index.js":
          'const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["./main.js","./main.css"])))=>i.map(i=>d[i]); window.location.protocol!=="file:"&&preload(()=>import("./main.js"),__vite__mapDeps([0,1]),import.meta.url);',
        "main.js":
          'window.externalViteLeafMarker = new URL("./logo.png", import.meta.url).href;',
        "main.css":
          '.external-vite-css-marker { background-image: url("./logo.png"); }',
        "logo.png": "external-vite-fixture-image",
      },
    );
    const distributionFiles = [
      "index.html",
      "assets/index.js",
      "assets/main.js",
      "assets/main.css",
      "assets/logo.png",
    ];
    const distributionBefore = new Map(
      await Promise.all(
        distributionFiles.map(
          async (fileName) =>
            [
              fileName,
              await readFile(
                path.join(fixture.distributionDirectory, fileName),
              ),
            ] as const,
        ),
      ),
    );

    const result = await buildV2Preview(fixture);
    const standalone = await readFile(fixture.outputPath, "utf8");

    expect(result).toMatchObject({
      scriptEntryCount: 1,
      stylesheetEntryCount: 1,
    });
    expect(standalone).toContain("external-vite-css-marker");
    expect(standalone).toContain("externalViteLeafMarker");
    expect(standalone).toContain("data:image/png;base64,");
    expect(standalone.indexOf("external-vite-css-marker")).toBeLessThan(
      standalone.indexOf("externalViteLeafMarker"),
    );
    expect(standalone).not.toContain("__vite__mapDeps");
    expect(standalone).not.toContain("./assets/index.js");
    expect(standalone).not.toContain("./main.js");
    expect(standalone).not.toContain("./main.css");
    expect(standalone).not.toContain("./logo.png");
    expect(standalone).not.toMatch(/<script\b[^>]*\bsrc\s*=/i);
    for (const fileName of distributionFiles) {
      expect(
        await readFile(path.join(fixture.distributionDirectory, fileName)),
      ).toEqual(distributionBefore.get(fileName));
    }
  });

  it("fails fast when the known external Vite entry points to a chunked leaf graph", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script type="module" src="./assets/index.js"></script>
</head><body></body></html>`,
      {
        "index.js":
          'const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["./main.js","./main.css"])))=>i.map(i=>d[i]); preload(()=>import("./main.js"),__vite__mapDeps([0,1]),import.meta.url);',
        "main.js": 'import "./chunk.js"; window.mainLoaded = true;',
        "main.css": "body { color: black; }",
        "chunk.js": "window.chunkLoaded = true;",
      },
    );

    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /chunked module graphs are unsupported/i,
    );
    await expect(readFile(fixture.outputPath)).rejects.toThrow();
  });

  it("fails fast when an external Vite bootstrap is not the only executable module entry", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script type="module" src="./assets/index.js"></script>
  <script type="module" src="./assets/second.js"></script>
</head><body></body></html>`,
      {
        "index.js":
          'const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["./main.js","./main.css"])))=>i.map(i=>d[i]); preload(()=>import("./main.js"),__vite__mapDeps([0,1]),import.meta.url);',
        "main.js": "window.mainLoaded = true;",
        "main.css": "body { color: black; }",
        "second.js": "window.secondLoaded = true;",
      },
    );

    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /only executable module entry/i,
    );
    await expect(readFile(fixture.outputPath)).rejects.toThrow();
  });

  it("fails fast when the external Vite leaf retains import.meta.url", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script type="module" src="./assets/index.js"></script>
</head><body></body></html>`,
      {
        "index.js":
          'const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["./main.js","./main.css"])))=>i.map(i=>d[i]); preload(()=>import("./main.js"),__vite__mapDeps([0,1]),import.meta.url);',
        "main.js": "window.moduleUrl = import.meta.url;",
        "main.css": "body { color: black; }",
      },
    );

    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /retains import\.meta\.url/i,
    );
    await expect(readFile(fixture.outputPath)).rejects.toThrow();
  });

  it("fails fast for an unknown inline module graph", async () => {
    const indexHtml = `<!doctype html><html><head>
  <script type="module">window.unknownInlineModule = true;</script>
</head><body></body></html>`;

    expect(() => parseDistributionEntryAssets(indexHtml)).toThrow(
      /unsupported inline module/i,
    );
    const fixture = await createDistributionFixture(indexHtml, {});
    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /unsupported inline module/i,
    );
  });

  it.each([
    ["static", 'import "./chunk.js"; window.entryLoaded = true;'],
    ["dynamic", 'window.entryLoaded = true; void import("./chunk.js");'],
  ])("fails fast for a %s local module import", async (_kind, entry) => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script type="module" src="./assets/entry.js"></script>
</head><body></body></html>`,
      {
        "entry.js": entry,
        "chunk.js": "window.chunkLoaded = true;",
      },
    );

    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /chunked module graphs are unsupported/i,
    );
    await expect(readFile(fixture.outputPath)).rejects.toThrow();
  });

  it("rejects local entry paths that escape dist", async () => {
    const fixture = await createDistributionFixture(
      `<!doctype html><html><head>
  <script type="module" src="../outside.js"></script>
</head><body></body></html>`,
      {},
    );

    await expect(
      resolveDistributionAsset(
        fixture.distributionDirectory,
        "../outside.js",
      ),
    ).rejects.toThrow(/escapes the distribution directory/i);
    await expect(buildV2Preview(fixture)).rejects.toThrow(
      /escapes the distribution directory/i,
    );
  });

  it.skipIf(process.platform === "win32")(
    "rejects an in-dist symlink that resolves outside dist",
    async () => {
      const fixture = await createDistributionFixture(
        `<!doctype html><html><head>
  <script type="module" src="./assets/linked-entry.js"></script>
</head><body></body></html>`,
        {},
      );
      const outsidePath = path.join(
        path.dirname(fixture.distributionDirectory),
        "outside-entry.js",
      );
      const linkedPath = path.join(
        fixture.distributionDirectory,
        "assets/linked-entry.js",
      );
      await writeFile(outsidePath, "window.outsideEntry = true;");
      await symlink(outsidePath, linkedPath);

      await expect(buildV2Preview(fixture)).rejects.toThrow(
        /escapes the distribution directory/i,
      );
    },
  );
});
