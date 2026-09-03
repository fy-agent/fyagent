import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import {
  V2_ROUTE_ENTRIES,
  verifyV2RouteChunks,
} from "../../../scripts/verify-v2-route-chunks.mjs";

const temporaryDirectories: string[] = [];

async function fixture() {
  const root = await mkdtemp(path.join(os.tmpdir(), "fyagent-v2-chunks-"));
  temporaryDirectories.push(root);
  await mkdir(path.join(root, ".vite"), { recursive: true });
  await mkdir(path.join(root, "assets"), { recursive: true });

  const manifest: Record<string, Record<string, unknown>> = {
    "index.html": {
      file: "assets/index.js",
      isEntry: true,
      dynamicImports: ["_main.js"],
    },
    "_main.js": {
      file: "assets/main.js",
      imports: ["_vendor.js", "index.html"],
      dynamicImports: [...V2_ROUTE_ENTRIES],
      css: ["assets/main.css"],
    },
    "_vendor.js": {
      file: "assets/vendor.js",
    },
  };
  for (const [index, route] of V2_ROUTE_ENTRIES.entries()) {
    manifest[route] = {
      file: `assets/route-${index}.js`,
      isDynamicEntry: true,
    };
    await writeFile(path.join(root, `assets/route-${index}.js`), "route");
  }
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

describe("verifyV2RouteChunks", () => {
  it("accepts seven distinct route chunks outside the bounded initial graph", async () => {
    const distributionDirectory = await fixture();
    const result = await verifyV2RouteChunks({ distributionDirectory });

    expect(result.routeChunks).toHaveLength(7);
    expect(result.initialChunks.map((chunk) => chunk.file).sort()).toEqual([
      "assets/index.js",
      "assets/main.js",
      "assets/vendor.js",
    ]);
  });

  it("rejects an initial chunk that exceeds the reviewed budget", async () => {
    const distributionDirectory = await fixture();

    await expect(
      verifyV2RouteChunks({
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
