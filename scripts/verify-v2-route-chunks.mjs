import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");

export const V2_ROUTE_ENTRIES = Object.freeze([
  "v2/pages/agents/Page.tsx",
  "v2/pages/auth/Page.tsx",
  "v2/pages/models/Page.tsx",
  "v2/pages/skills/Page.tsx",
  "v2/pages/mcp/Page.tsx",
  "v2/pages/prompts/Page.tsx",
  "v2/pages/memory/Page.tsx",
]);

export const V2_BUILD_BUDGET = Object.freeze({
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
    throw new Error(`V2 build manifest is missing a valid record: ${key}`);
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
    throw new Error(`V2 build asset escaped dist: ${relativePath}`);
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

export async function verifyV2RouteChunks({
  distributionDirectory = path.join(repositoryRoot, "dist"),
  budget = V2_BUILD_BUDGET,
} = {}) {
  const manifestPath = path.join(
    distributionDirectory,
    ".vite",
    "manifest.json",
  );
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  const entry = assertManifestRecord(manifest, "index.html");
  const bootstrapImports = entry.dynamicImports ?? [];
  if (bootstrapImports.length !== 1) {
    throw new Error(
      "V2 build must expose exactly one reviewed bootstrap chunk",
    );
  }

  const bootstrapKey = bootstrapImports[0];
  const bootstrap = assertManifestRecord(manifest, bootstrapKey);
  const dynamicRoutes = new Set(bootstrap.dynamicImports ?? []);
  if (
    dynamicRoutes.size !== V2_ROUTE_ENTRIES.length ||
    V2_ROUTE_ENTRIES.some((route) => !dynamicRoutes.has(route))
  ) {
    throw new Error(
      `V2 bootstrap must dynamically import exactly ${V2_ROUTE_ENTRIES.length} product pages`,
    );
  }

  const routeFiles = new Set();
  const routeChunks = [];
  for (const route of V2_ROUTE_ENTRIES) {
    const record = assertManifestRecord(manifest, route);
    if (record.isDynamicEntry !== true || !record.file.endsWith(".js")) {
      throw new Error(
        `V2 product page is not a JavaScript dynamic entry: ${route}`,
      );
    }
    if (routeFiles.has(record.file)) {
      throw new Error(`V2 product pages share an entry chunk: ${record.file}`);
    }
    routeFiles.add(record.file);
    const bytes = await assetSize(distributionDirectory, record.file);
    if (bytes > budget.routeChunkBytes) {
      throw new Error(
        `V2 route chunk exceeds ${budget.routeChunkBytes} bytes: ${route} (${bytes})`,
      );
    }
    routeChunks.push({ route, file: record.file, bytes });
  }

  const initialKeys = collectStaticClosure(manifest, [
    "index.html",
    bootstrapKey,
  ]);
  for (const route of V2_ROUTE_ENTRIES) {
    if (initialKeys.has(route)) {
      throw new Error(
        `V2 product page leaked into the initial graph: ${route}`,
      );
    }
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
        `V2 initial chunk exceeds ${budget.initialChunkBytes} bytes: ${record.file} (${bytes})`,
      );
    }
    initialJavaScriptBytes += bytes;
    initialChunks.push({ key, file: record.file, bytes });
  }
  if (initialJavaScriptBytes > budget.initialJavaScriptBytes) {
    throw new Error(
      `V2 initial JavaScript exceeds ${budget.initialJavaScriptBytes} bytes (${initialJavaScriptBytes})`,
    );
  }

  let initialCssBytes = 0;
  for (const css of cssFiles) {
    initialCssBytes += await assetSize(distributionDirectory, css);
  }
  if (initialCssBytes > budget.initialCssBytes) {
    throw new Error(
      `V2 initial CSS exceeds ${budget.initialCssBytes} bytes (${initialCssBytes})`,
    );
  }

  return {
    initialJavaScriptBytes,
    initialCssBytes,
    initialChunks,
    routeChunks,
  };
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const result = await verifyV2RouteChunks();
  console.log(
    `V2 route chunks verified: ${result.routeChunks.length} routes, ` +
      `${result.initialJavaScriptBytes} initial JS bytes, ` +
      `${result.initialCssBytes} initial CSS bytes.`,
  );
}
