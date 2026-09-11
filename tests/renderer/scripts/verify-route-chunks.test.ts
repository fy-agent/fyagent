import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import {
  RENDERER_ROUTE_ENTRIES,
  RENDERER_DEFERRED_PORT_ENTRIES,
  verifyRouteChunks,
} from "../../../scripts/verify-route-chunks.mjs";

const temporaryDirectories: string[] = [];

async function fixture(
  patch?: (manifest: Record<string, Record<string, unknown>>) => void,
) {
  const root = await mkdtemp(path.join(os.tmpdir(), "fyagent-chunks-"));
  temporaryDirectories.push(root);
  await mkdir(path.join(root, ".vite"), { recursive: true });
  await mkdir(path.join(root, "assets"), { recursive: true });

  const manifest: Record<string, Record<string, unknown>> = {
    "index.html": {
      file: "assets/index.js",
      isEntry: true,
      imports: ["_main.js"],
    },
    "_main.js": {
      file: "assets/main.js",
      imports: ["_vendor.js", "index.html"],
      dynamicImports: [
        ...RENDERER_ROUTE_ENTRIES,
        ...RENDERER_DEFERRED_PORT_ENTRIES,
      ],
      css: ["assets/main.css"],
    },
    "_vendor.js": {
      file: "assets/vendor.js",
    },
  };
  for (const [index, route] of RENDERER_ROUTE_ENTRIES.entries()) {
    manifest[route] = {
      file: `assets/route-${index}.js`,
      isDynamicEntry: true,
    };
    await writeFile(path.join(root, `assets/route-${index}.js`), "route");
  }
  for (const [index, key] of RENDERER_DEFERRED_PORT_ENTRIES.entries()) {
    manifest[key] = {
      file: `assets/port-${index}.js`,
      isDynamicEntry: true,
    };
    await writeFile(
      path.join(root, `assets/port-${index}.js`),
      "deferred port",
    );
  }
  patch?.(manifest);
  await Promise.all([
    writeFile(path.join(root, "assets/index.js"), "entry"),
    writeFile(path.join(root, "assets/main.js"), "main"),
    writeFile(path.join(root, "assets/vendor.js"), "vendor"),
    writeFile(path.join(root, "assets/main.css"), "css"),
    writeFile(path.join(root, ".vite/manifest.json"), JSON.stringify(manifest)),
  ]);
  return root;
}

afterEach(async () => {
  await Promise.all(
    temporaryDirectories
      .splice(0)
      .map((directory) => rm(directory, { recursive: true, force: true })),
  );
});

describe("verifyRouteChunks", () => {
  it("accepts eight distinct routes and the deferred health port outside the bounded initial graph", async () => {
    const distributionDirectory = await fixture();
    const result = await verifyRouteChunks({ distributionDirectory });

    expect(result.routeChunks).toHaveLength(8);
    expect(result.deferredPortChunks).toHaveLength(1);
    expect(result.initialChunks.map((chunk) => chunk.file).sort()).toEqual([
      "assets/index.js",
      "assets/main.js",
      "assets/vendor.js",
    ]);
  });

  it.each([RENDERER_ROUTE_ENTRIES[0], RENDERER_DEFERRED_PORT_ENTRIES[0]])(
    "rejects %s if it leaks into the initial graph",
    async (key) => {
      const distributionDirectory = await fixture((manifest) => {
        manifest["_vendor.js"].imports = [key];
      });
      await expect(
        verifyRouteChunks({ distributionDirectory }),
      ).rejects.toThrow("leaked into the initial graph");
    },
  );

  it("rejects an unreviewed dynamic entry", async () => {
    const distributionDirectory = await fixture((manifest) => {
      manifest["_vendor.js"].dynamicImports = ["unexpected.ts"];
    });
    await expect(verifyRouteChunks({ distributionDirectory })).rejects.toThrow(
      "must dynamically import exactly",
    );
  });

  it("applies the unchanged route chunk budget to the deferred port", async () => {
    const distributionDirectory = await fixture();
    await expect(
      verifyRouteChunks({
        distributionDirectory,
        budget: {
          initialJavaScriptBytes: 100,
          initialChunkBytes: 100,
          initialCssBytes: 100,
          routeChunkBytes: 5,
        },
      }),
    ).rejects.toThrow("deferred port exceeds");
  });

  it("rejects an initial chunk that exceeds the reviewed budget", async () => {
    const distributionDirectory = await fixture();

    await expect(
      verifyRouteChunks({
        distributionDirectory,
        budget: {
          initialJavaScriptBytes: 100,
          initialChunkBytes: 3,
          initialCssBytes: 100,
          routeChunkBytes: 100,
        },
      }),
    ).rejects.toThrow("initial chunk exceeds");
  });
});
