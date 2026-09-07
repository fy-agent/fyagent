import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");

export const RENDERER_ROUTE_ENTRIES = Object.freeze([
  "pages/agents/Page.tsx",
  "pages/auth/Page.tsx",
  "pages/models/Page.tsx",
  "pages/skills/Page.tsx",
  "pages/mcp/Page.tsx",
  "pages/prompts/Page.tsx",
  "pages/memory/Page.tsx",
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
  const dynamicRoutes = new Set(
    [...initialKeys].flatMap((key) => manifest[key].dynamicImports ?? []),
  );
  if (
    dynamicRoutes.size !== RENDERER_ROUTE_ENTRIES.length ||
    RENDERER_ROUTE_ENTRIES.some((route) => !dynamicRoutes.has(route))
  ) {
    throw new Error(
      `Renderer bootstrap must dynamically import exactly ${RENDERER_ROUTE_ENTRIES.length} product pages`,
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

  for (const route of RENDERER_ROUTE_ENTRIES) {
    if (initialKeys.has(route)) {
      throw new Error(
        `Renderer product page leaked into the initial graph: ${route}`,
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
