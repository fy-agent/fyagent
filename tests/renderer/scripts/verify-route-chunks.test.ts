import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import {
  RENDERER_ROUTE_ENTRIES,
  RENDERER_DEFERRED_PORT_ENTRIES,
  RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES,
  RENDERER_DEFERRED_SHELL_ENTRIES,
  RENDERER_NESTED_SUBSCRIPTION_PORT,
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
        ...RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES,
        ...RENDERER_DEFERRED_SHELL_ENTRIES,
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
  for (const [index, key] of RENDERER_DEFERRED_SHELL_ENTRIES.entries()) {
    manifest[key] = { file: `assets/shell-${index}.js`, isDynamicEntry: true };
    await writeFile(path.join(root, `assets/shell-${index}.js`), "shell");
  }
  manifest[RENDERER_NESTED_SUBSCRIPTION_PORT.importer].dynamicImports = [
    RENDERER_NESTED_SUBSCRIPTION_PORT.entry,
  ];
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
  it("accepts nine distinct routes and the reviewed deferred ports outside the bounded initial graph", async () => {
    const distributionDirectory = await fixture();
    const result = await verifyRouteChunks({ distributionDirectory });

    expect(result.routeChunks).toHaveLength(9);
    expect(result.routeChunks.map(({ route }) => route)).toContain(
      "app/ProjectsWorkspace.tsx",
    );
    expect(result.deferredPortChunks).toHaveLength(8);
    expect(result.initialChunks.map((chunk) => chunk.file).sort()).toEqual([
      "assets/index.js",
      "assets/main.js",
      "assets/vendor.js",
    ]);
  });

  it.each([
    RENDERER_ROUTE_ENTRIES[0],
    ...RENDERER_DEFERRED_PORT_ENTRIES,
    ...RENDERER_DEFERRED_SHELL_ENTRIES,
  ])("rejects %s if it leaks into the initial graph", async (key) => {
    const distributionDirectory = await fixture((manifest) => {
      manifest["_vendor.js"].imports = [key];
    });
    await expect(verifyRouteChunks({ distributionDirectory })).rejects.toThrow(
      "leaked into the initial graph",
    );
  });

  it("rejects a missing Projects composition entry", async () => {
    const distributionDirectory = await fixture((manifest) => {
      manifest["_main.js"].dynamicImports = [
        ...RENDERER_ROUTE_ENTRIES.filter(
          (entry) => entry !== "app/ProjectsWorkspace.tsx",
        ),
        ...RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES,
        ...RENDERER_DEFERRED_SHELL_ENTRIES,
      ];
    });
    await expect(verifyRouteChunks({ distributionDirectory })).rejects.toThrow(
      "must dynamically import exactly 9 product pages, 7 deferred ports and 1 shell dialogs",
    );
  });

  it("rejects Projects sharing another primary route entry chunk", async () => {
    const distributionDirectory = await fixture((manifest) => {
      manifest["app/ProjectsWorkspace.tsx"].file =
        manifest["pages/agents/Page.tsx"].file;
    });
    await expect(verifyRouteChunks({ distributionDirectory })).rejects.toThrow(
      "product pages share an entry chunk",
    );
  });

  it("rejects an unreviewed dynamic entry", async () => {
    const distributionDirectory = await fixture((manifest) => {
      manifest["_vendor.js"].dynamicImports = ["unexpected.ts"];
    });
    await expect(verifyRouteChunks({ distributionDirectory })).rejects.toThrow(
      "must dynamically import exactly",
    );
  });

  it.each([
    [],
    ["unexpected.ts"],
    [RENDERER_NESTED_SUBSCRIPTION_PORT.entry, "unexpected.ts"],
  ])(
    "rejects missing or unreviewed nested subscription entries: %j",
    async (...entries) => {
      const distributionDirectory = await fixture((manifest) => {
        manifest[RENDERER_NESTED_SUBSCRIPTION_PORT.importer].dynamicImports =
          entries;
      });
      await expect(
        verifyRouteChunks({ distributionDirectory }),
      ).rejects.toThrow(
        "Models port must dynamically import only the subscription port",
      );
    },
  );

  it("rejects moving the subscription port into Models static imports", async () => {
    const distributionDirectory = await fixture((manifest) => {
      manifest[RENDERER_NESTED_SUBSCRIPTION_PORT.importer].imports = [
        RENDERER_NESTED_SUBSCRIPTION_PORT.entry,
      ];
    });
    await expect(verifyRouteChunks({ distributionDirectory })).rejects.toThrow(
      "subscription port must remain deferred from Models",
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
