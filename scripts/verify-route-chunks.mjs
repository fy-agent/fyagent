import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");

export const RENDERER_ROUTE_ENTRIES = Object.freeze([
  "app/ProjectsWorkspace.tsx",
  "pages/agents/Page.tsx",
  "pages/health/Page.tsx",
  "pages/auth/Page.tsx",
  "pages/models/Page.tsx",
  "pages/skills/Page.tsx",
  "pages/mcp/Page.tsx",
  "pages/prompts/Page.tsx",
  "pages/memory/Page.tsx",
]);

// These capability adapters are loaded only when their port is first used.
// Keep the list explicit: additional lazy entries still require review.
export const RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES = Object.freeze([
  "shared/platform/tauri/feature-ports/health.ts",
  "shared/platform/tauri/feature-ports/projects.ts",
  "shared/platform/tauri/feature-ports/delivery-kits.ts",
  "shared/platform/tauri/feature-ports/verification.ts",
  "shared/platform/tauri/feature-ports/models.ts",
  "shared/platform/tauri/feature-ports/configRecovery.ts",
]);

export const RENDERER_NESTED_SUBSCRIPTION_PORT = Object.freeze({
  importer: "shared/platform/tauri/feature-ports/models.ts",
  entry: "shared/platform/tauri/feature-ports/managedSubscriptions.ts",
});

export const RENDERER_DEFERRED_PORT_ENTRIES = Object.freeze([
  ...RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES,
  RENDERER_NESTED_SUBSCRIPTION_PORT.entry,
]);

export const RENDERER_BUILD_BUDGET = Object.freeze({
  initialJavaScriptBytes: 650 * 1024,
  initialChunkBytes: 300 * 1024,
  initialCssBytes: 64 * 1024,
  routeChunkBytes: 180 * 1024,
});

function assertManifestRecord(manifest, key) {
  const record = manifest[key];
  if (
    !record ||
    typeof record !== "object" ||
    typeof record.file !== "string"
  ) {
    throw new Error(
      `Renderer build manifest is missing a valid record: ${key}`,
    );
  }
  return record;
}

async function assetSize(distributionDirectory, relativePath) {
  const absolutePath = path.resolve(distributionDirectory, relativePath);
  const relative = path.relative(distributionDirectory, absolutePath);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error(`Renderer build asset escaped dist: ${relativePath}`);
  }
  return (await stat(absolutePath)).size;
}

function collectStaticClosure(manifest, roots) {
  const closure = new Set();
  const queue = [...roots];
  while (queue.length > 0) {
    const key = queue.shift();
    if (closure.has(key)) continue;
    const record = assertManifestRecord(manifest, key);
    closure.add(key);
    for (const dependency of record.imports ?? []) queue.push(dependency);
  }
  return closure;
}

export async function verifyRouteChunks({
  distributionDirectory = path.join(repositoryRoot, "dist"),
  budget = RENDERER_BUILD_BUDGET,
} = {}) {
  const manifestPath = path.join(
    distributionDirectory,
    ".vite",
    "manifest.json",
  );
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  const entry = assertManifestRecord(manifest, "index.html");
  const initialKeys = collectStaticClosure(manifest, ["index.html"]);
  for (const key of [
    ...RENDERER_ROUTE_ENTRIES,
    ...RENDERER_DEFERRED_PORT_ENTRIES,
  ]) {
    if (initialKeys.has(key)) {
      throw new Error(
        `Renderer deferred entry leaked into the initial graph: ${key}`,
      );
    }
  }
  const dynamicEntries = new Set(
    [...initialKeys].flatMap((key) => manifest[key].dynamicImports ?? []),
  );
  const expectedDynamicEntries = [
    ...RENDERER_ROUTE_ENTRIES,
    ...RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES,
  ];
  if (
    dynamicEntries.size !== expectedDynamicEntries.length ||
    expectedDynamicEntries.some((key) => !dynamicEntries.has(key))
  ) {
    throw new Error(
      `Renderer bootstrap must dynamically import exactly ${RENDERER_ROUTE_ENTRIES.length} product pages and ${RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES.length} deferred ports`,
    );
  }

  const nested = RENDERER_NESTED_SUBSCRIPTION_PORT;
  const modelPort = assertManifestRecord(manifest, nested.importer);
  const nestedEntries = new Set(modelPort.dynamicImports ?? []);
  if (nestedEntries.size !== 1 || !nestedEntries.has(nested.entry)) {
    throw new Error(
      "Renderer Models port must dynamically import only the subscription port",
    );
  }
  if (collectStaticClosure(manifest, [nested.importer]).has(nested.entry)) {
    throw new Error(
      "Renderer subscription port must remain deferred from Models",
    );
  }

  const routeFiles = new Set();
  const routeChunks = [];
  for (const route of RENDERER_ROUTE_ENTRIES) {
    const record = assertManifestRecord(manifest, route);
    if (record.isDynamicEntry !== true || !record.file.endsWith(".js")) {
      throw new Error(
        `Renderer product page is not a JavaScript dynamic entry: ${route}`,
      );
    }
    if (routeFiles.has(record.file)) {
      throw new Error(
        `Renderer product pages share an entry chunk: ${record.file}`,
      );
    }
    routeFiles.add(record.file);
    const bytes = await assetSize(distributionDirectory, record.file);
    if (bytes > budget.routeChunkBytes) {
      throw new Error(
        `Renderer route chunk exceeds ${budget.routeChunkBytes} bytes: ${route} (${bytes})`,
      );
    }
    routeChunks.push({ route, file: record.file, bytes });
  }

  const deferredPortChunks = [];
  for (const key of RENDERER_DEFERRED_PORT_ENTRIES) {
    const record = assertManifestRecord(manifest, key);
    if (record.isDynamicEntry !== true || !record.file.endsWith(".js")) {
      throw new Error(`Renderer deferred port is not a dynamic entry: ${key}`);
    }
    const bytes = await assetSize(distributionDirectory, record.file);
    if (bytes > budget.routeChunkBytes) {
      throw new Error(
        `Renderer deferred port exceeds ${budget.routeChunkBytes} bytes: ${key} (${bytes})`,
      );
    }
    deferredPortChunks.push({ key, file: record.file, bytes });
  }

  let initialJavaScriptBytes = 0;
  const initialChunks = [];
  const cssFiles = new Set();
  for (const key of initialKeys) {
    const record = assertManifestRecord(manifest, key);
    for (const css of record.css ?? []) cssFiles.add(css);
    if (!record.file.endsWith(".js")) continue;
    const bytes = await assetSize(distributionDirectory, record.file);
    if (bytes > budget.initialChunkBytes) {
      throw new Error(
        `Renderer initial chunk exceeds ${budget.initialChunkBytes} bytes: ${record.file} (${bytes})`,
      );
    }
    initialJavaScriptBytes += bytes;
    initialChunks.push({ key, file: record.file, bytes });
  }
  if (initialJavaScriptBytes > budget.initialJavaScriptBytes) {
    throw new Error(
      `Renderer initial JavaScript exceeds ${budget.initialJavaScriptBytes} bytes (${initialJavaScriptBytes})`,
    );
  }

  let initialCssBytes = 0;
  for (const css of cssFiles) {
    initialCssBytes += await assetSize(distributionDirectory, css);
  }
  if (initialCssBytes > budget.initialCssBytes) {
    throw new Error(
      `Renderer initial CSS exceeds ${budget.initialCssBytes} bytes (${initialCssBytes})`,
    );
  }

  return {
    initialJavaScriptBytes,
    initialCssBytes,
    initialChunks,
    routeChunks,
    deferredPortChunks,
  };
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const result = await verifyRouteChunks();
  console.log(
    `Renderer route chunks verified: ${result.routeChunks.length} routes, ` +
      `${result.initialJavaScriptBytes} initial JS bytes, ` +
      `${result.initialCssBytes} initial CSS bytes.`,
  );
}
