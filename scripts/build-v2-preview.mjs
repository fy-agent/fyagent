import { readFile, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptDirectory = path.dirname(scriptPath);
const repositoryRoot = path.resolve(scriptDirectory, "..");

const defaultDistributionDirectory = path.join(repositoryRoot, "dist");
const defaultOutputPath = path.join(
  repositoryRoot,
  "FyAgent-前端交互预览.html",
);

function htmlAttribute(tag, name) {
  const pattern = new RegExp(
    `\\b${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s"'=<>` +
      "`" +
      "]+))",
    "i",
  );
  const match = tag.match(pattern);
  return match ? (match[1] ?? match[2] ?? match[3] ?? "") : undefined;
}

export function parseDistributionEntryAssets(indexHtml) {
  const entries = [];
  const tagPattern =
    /<script\b[^>]*>[\s\S]*?<\/script\s*>|<link\b[^>]*>/gi;

  for (const match of indexHtml.matchAll(tagPattern)) {
    const tag = match[0];
    const start = match.index ?? 0;

    if (/^<script\b/i.test(tag)) {
      const type = htmlAttribute(tag, "type")?.toLocaleLowerCase();
      const source = htmlAttribute(tag, "src");
      if (type === "module" && source) {
        entries.push({
          kind: "script",
          source,
          start,
          end: start + tag.length,
        });
      } else if (type === "module") {
        const content = tag
          .replace(/^<script\b[^>]*>/i, "")
          .replace(/<\/script\s*>$/i, "");
        const bootstrap = parseViteBootstrapEntries(content);
        if (!bootstrap) {
          throw new Error(
            "Unsupported inline module in dist/index.html; expected the known Vite bootstrap shape.",
          );
        }
        entries.push({
          kind: "vite-bootstrap",
          ...bootstrap,
          start,
          end: start + tag.length,
        });
      }
      continue;
    }

    const relations = (htmlAttribute(tag, "rel") ?? "")
      .toLocaleLowerCase()
      .split(/\s+/);
    const source = htmlAttribute(tag, "href");
    if (relations.includes("stylesheet") && source) {
      entries.push({
        kind: "stylesheet",
        source,
        start,
        end: start + tag.length,
      });
    }
  }

  if (
    !entries.some(
      (entry) =>
        entry.kind === "script" || entry.kind === "vite-bootstrap",
    )
  ) {
    throw new Error(
      "Unable to locate a direct module entry script in dist/index.html.",
    );
  }

  return entries;
}

export function parseViteBootstrapEntries(content) {
  if (!content.includes("__vite__mapDeps")) {
    return undefined;
  }

  const dependencyArrayMatch = content.match(
    /__vite__mapDeps[\s\S]*?([A-Za-z_$][\w$]*)\.f\s*\|\|\s*\(\s*\1\.f\s*=\s*(\[[^\]]*\])\s*\)/,
  );
  if (!dependencyArrayMatch) {
    throw new Error("Unsupported Vite bootstrap dependency map.");
  }

  let dependencySources;
  try {
    dependencySources = JSON.parse(dependencyArrayMatch[2]);
  } catch {
    throw new Error("Unsupported Vite bootstrap dependency array.");
  }
  if (
    !Array.isArray(dependencySources) ||
    dependencySources.some((source) => typeof source !== "string")
  ) {
    throw new Error("Unsupported Vite bootstrap dependency array.");
  }

  const dynamicImports = [
    ...content.matchAll(/\bimport\s*\(\s*(["'])([^"']+)\1\s*\)/g),
  ].map((match) => match[2]);
  const scriptSources = dependencySources.filter((source) =>
    /\.m?js(?:[?#].*)?$/i.test(source),
  );
  const stylesheetSources = dependencySources.filter((source) =>
    /\.css(?:[?#].*)?$/i.test(source),
  );

  if (
    dependencySources.some(
      (source) =>
        !isLocalModuleSource(source) ||
        !/\.(?:m?js|css)(?:[?#].*)?$/i.test(source),
    ) ||
    dynamicImports.length !== 1 ||
    scriptSources.length !== 1 ||
    dynamicImports[0] !== scriptSources[0] ||
    !dependencySources.includes(dynamicImports[0])
  ) {
    throw new Error(
      "Unsupported Vite bootstrap graph; expected one mapped local JS entry and its CSS dependencies.",
    );
  }

  return {
    scriptSource: scriptSources[0],
    stylesheetSources,
  };
}

function isRemoteReference(source) {
  return (
    /^(?:[a-z][a-z\d+.-]*:)?\/\//i.test(source) ||
    /^(?:data|blob|javascript):/i.test(source)
  );
}

function assertContainedPath(rootPath, targetPath, source) {
  const relativePath = path.relative(rootPath, targetPath);
  if (
    relativePath === "" ||
    relativePath === ".." ||
    relativePath.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relativePath)
  ) {
    throw new Error(
      `Entry asset escapes the distribution directory: ${source}`,
    );
  }
}

export async function resolveDistributionAsset(
  distributionDirectory,
  source,
  relativeToDirectory = distributionDirectory,
) {
  const trimmedSource = source.trim();
  if (!trimmedSource || isRemoteReference(trimmedSource)) {
    throw new Error(`Entry asset must be a local path: ${source}`);
  }

  const sourceWithoutQuery = trimmedSource.split(/[?#]/, 1)[0];
  let decodedSource;
  try {
    decodedSource = decodeURIComponent(sourceWithoutQuery);
  } catch {
    throw new Error(`Entry asset has an invalid encoded path: ${source}`);
  }

  const distributionPath = path.resolve(distributionDirectory);
  const [canonicalDistributionPath, canonicalRelativeDirectory] =
    await Promise.all([
      realpath(distributionPath),
      realpath(path.resolve(relativeToDirectory)),
    ]);
  const resolvedPath = decodedSource.startsWith("/")
    ? path.resolve(
        canonicalDistributionPath,
        decodedSource.replace(/^\/+/, ""),
      )
    : path.resolve(canonicalRelativeDirectory, decodedSource);

  assertContainedPath(canonicalDistributionPath, resolvedPath, source);

  const canonicalAssetPath = await realpath(resolvedPath);
  assertContainedPath(
    canonicalDistributionPath,
    canonicalAssetPath,
    source,
  );

  return canonicalAssetPath;
}

async function assertOutputOutsideDistribution(
  distributionDirectory,
  outputPath,
) {
  const distributionPath = path.resolve(distributionDirectory);
  const resolvedOutputPath = path.resolve(outputPath);
  const lexicalRelativePath = path.relative(
    distributionPath,
    resolvedOutputPath,
  );
  if (
    lexicalRelativePath === "" ||
    (!lexicalRelativePath.startsWith(`..${path.sep}`) &&
      lexicalRelativePath !== ".." &&
      !path.isAbsolute(lexicalRelativePath))
  ) {
    throw new Error("Standalone output must be outside the dist directory.");
  }

  const canonicalDistributionPath = await realpath(distributionPath);
  let canonicalOutputPath;
  try {
    canonicalOutputPath = await realpath(resolvedOutputPath);
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
    canonicalOutputPath = path.join(
      await realpath(path.dirname(resolvedOutputPath)),
      path.basename(resolvedOutputPath),
    );
  }
  const canonicalRelativePath = path.relative(
    canonicalDistributionPath,
    canonicalOutputPath,
  );
  if (
    canonicalRelativePath === "" ||
    (!canonicalRelativePath.startsWith(`..${path.sep}`) &&
      canonicalRelativePath !== ".." &&
      !path.isAbsolute(canonicalRelativePath))
  ) {
    throw new Error("Standalone output must be outside the dist directory.");
  }
}

function escapeInlineScript(content) {
  return content.replace(/<\/script/gi, "<\\/script");
}

function escapeInlineStyle(content) {
  return content.replace(/<\/style/gi, "<\\/style");
}

function mimeTypeForAsset(assetPath) {
  const extension = path.extname(assetPath).toLocaleLowerCase();
  return (
    {
      ".avif": "image/avif",
      ".gif": "image/gif",
      ".ico": "image/x-icon",
      ".jpeg": "image/jpeg",
      ".jpg": "image/jpeg",
      ".otf": "font/otf",
      ".png": "image/png",
      ".svg": "image/svg+xml",
      ".ttf": "font/ttf",
      ".webp": "image/webp",
      ".woff": "font/woff",
      ".woff2": "font/woff2",
    }[extension] ?? "application/octet-stream"
  );
}

async function assetDataUrl(assetPath) {
  const asset = await readFile(assetPath);
  return `data:${mimeTypeForAsset(assetPath)};base64,${asset.toString(
    "base64",
  )}`;
}

async function replaceAsync(source, pattern, replacer) {
  let result = "";
  let cursor = 0;

  for (const match of source.matchAll(pattern)) {
    const start = match.index ?? 0;
    result += source.slice(cursor, start);
    result += await replacer(match);
    cursor = start + match[0].length;
  }

  return result + source.slice(cursor);
}

function shouldInlineReference(source) {
  return (
    source.length > 0 &&
    !source.startsWith("#") &&
    !isRemoteReference(source)
  );
}

function isLocalModuleSource(source) {
  return /^(?:\.{1,2}\/|\/)/.test(source);
}

function findLocalModuleImport(content) {
  const patterns = [
    /\bimport\s*\(\s*(["'])([^"']+)\1\s*\)/g,
    /\bimport\s+(?:(?:[\w$*{},\s]+)\s+from\s+)?(["'])([^"']+)\1/g,
    /\bexport\s+(?:\*|\{[^}]*\})\s+from\s+(["'])([^"']+)\1/g,
  ];

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      if (isLocalModuleSource(match[2])) {
        return match[2];
      }
    }
  }

  return undefined;
}

function assertSelfContainedDirectEntry(content, source) {
  const importedSource = findLocalModuleImport(content);
  if (importedSource) {
    throw new Error(
      `Direct module entry ${source} imports local module ${importedSource}; chunked module graphs are unsupported by the standalone builder.`,
    );
  }
}

function assertNoResidualImportMetaUrl(content, source) {
  if (/\bimport\s*\.\s*meta\s*\.\s*url\b/.test(content)) {
    throw new Error(
      `Module ${source} retains import.meta.url after asset inlining; the standalone builder cannot preserve that runtime reference.`,
    );
  }
}

async function inlineCssAssets(content, stylesheetPath, distributionDirectory) {
  return replaceAsync(
    content,
    /url\(\s*(?:(["'])(.*?)\1|([^\s)]+))\s*\)/gi,
    async (match) => {
      const source = match[2] ?? match[3] ?? "";
      if (!shouldInlineReference(source)) {
        return match[0];
      }

      const assetPath = await resolveDistributionAsset(
        distributionDirectory,
        source,
        path.dirname(stylesheetPath),
      );
      return `url("${await assetDataUrl(assetPath)}")`;
    },
  );
}

async function inlineJavaScriptAssets(
  content,
  scriptAssetPath,
  distributionDirectory,
) {
  return replaceAsync(
    content,
    /new URL\(\s*(["'])([^"']+\.(?:png|jpe?g|gif|webp|svg|ico|avif)(?:[?#][^"']*)?)\1\s*,\s*import\.meta\.url\s*\)/gi,
    async (match) => {
      const assetPath = await resolveDistributionAsset(
        distributionDirectory,
        match[2],
        path.dirname(scriptAssetPath),
      );
      return `new URL(${JSON.stringify(await assetDataUrl(assetPath))})`;
    },
  );
}

function replaceEntryTags(indexHtml, entries, replacements) {
  let result = "";
  let cursor = 0;

  entries.forEach((entry, index) => {
    result += indexHtml.slice(cursor, entry.start);
    result += replacements[index];
    cursor = entry.end;
  });

  return result + indexHtml.slice(cursor);
}

function renderInlineEntry(kind, content) {
  if (kind === "stylesheet") {
    return `<style data-fyagent-inline-entry="stylesheet">${escapeInlineStyle(
      content,
    )}</style>`;
  }
  return `<script type="module" data-fyagent-inline-entry="script">${escapeInlineScript(
    content,
  )}</script>`;
}

async function inlineKnownViteBootstrap(
  bootstrap,
  distributionDirectory,
  relativeToDirectory,
) {
  const styles = [];
  for (const stylesheetSource of bootstrap.stylesheetSources) {
    const stylesheetPath = await resolveDistributionAsset(
      distributionDirectory,
      stylesheetSource,
      relativeToDirectory,
    );
    styles.push(
      renderInlineEntry(
        "stylesheet",
        await inlineCssAssets(
          await readFile(stylesheetPath, "utf8"),
          stylesheetPath,
          distributionDirectory,
        ),
      ),
    );
  }

  const scriptPath = await resolveDistributionAsset(
    distributionDirectory,
    bootstrap.scriptSource,
    relativeToDirectory,
  );
  const scriptContent = await readFile(scriptPath, "utf8");
  assertSelfContainedDirectEntry(scriptContent, bootstrap.scriptSource);
  const inlinedScriptContent = await inlineJavaScriptAssets(
    scriptContent,
    scriptPath,
    distributionDirectory,
  );
  assertNoResidualImportMetaUrl(
    inlinedScriptContent,
    bootstrap.scriptSource,
  );
  const script = renderInlineEntry(
    "script",
    inlinedScriptContent,
  );

  return [...styles, script].join("\n");
}

async function inlineEntriesForStandalone(
  indexHtml,
  entries,
  distributionDirectory,
) {
  const replacements = [];
  let externalViteStylesheetEntryCount = 0;

  for (const entry of entries) {
    if (entry.kind === "vite-bootstrap") {
      replacements.push(
        await inlineKnownViteBootstrap(
          entry,
          distributionDirectory,
          distributionDirectory,
        ),
      );
      continue;
    }

    const assetPath = await resolveDistributionAsset(
      distributionDirectory,
      entry.source,
    );
    const content = await readFile(assetPath, "utf8");

    if (entry.kind === "stylesheet") {
      replacements.push(
        renderInlineEntry(
          "stylesheet",
          await inlineCssAssets(
            content,
            assetPath,
            distributionDirectory,
          ),
        ),
      );
      continue;
    }

    const externalViteBootstrap = parseViteBootstrapEntries(content);
    if (externalViteBootstrap) {
      const executableEntryCount = entries.filter(
        (candidate) =>
          candidate.kind === "script" ||
          candidate.kind === "vite-bootstrap",
      ).length;
      if (executableEntryCount !== 1) {
        throw new Error(
          "External Vite bootstrap must be the only executable module entry in dist/index.html.",
        );
      }
      externalViteStylesheetEntryCount +=
        externalViteBootstrap.stylesheetSources.length;
      replacements.push(
        await inlineKnownViteBootstrap(
          externalViteBootstrap,
          distributionDirectory,
          path.dirname(assetPath),
        ),
      );
      continue;
    }

    assertSelfContainedDirectEntry(content, entry.source);
    replacements.push(
      renderInlineEntry(
        "script",
        await inlineJavaScriptAssets(
          content,
          assetPath,
          distributionDirectory,
        ),
      ),
    );
  }

  return {
    html: replaceEntryTags(indexHtml, entries, replacements),
    externalViteStylesheetEntryCount,
  };
}

function stripDistributionFileRedirect(indexHtml) {
  return indexHtml.replace(
    /<script\b[^>]*>[\s\S]*?<\/script\s*>/gi,
    (tag) =>
      tag.includes("FyAgent-前端交互预览.html") &&
      tag.includes("window.location.protocol")
        ? ""
        : tag,
  );
}

function addStandaloneBootstrap(indexHtml) {
  const bootstrap = `
    <meta name="fyagent-preview" content="standalone-v2-native-preview" />
    <script>
      window.__FYAGENT_STANDALONE_PREVIEW__ = true;
      if (!window.location.hash) window.location.hash = "#/prompts";
    </script>`;

  if (!/<\/head\s*>/i.test(indexHtml)) {
    throw new Error("dist/index.html does not contain a closing head tag.");
  }
  return indexHtml.replace(/<\/head\s*>/i, `${bootstrap}\n  </head>`);
}

async function inlineHtmlImageLinks(indexHtml, distributionDirectory) {
  return replaceAsync(indexHtml, /<link\b[^>]*>/gi, async (match) => {
    const tag = match[0];
    const relations = (htmlAttribute(tag, "rel") ?? "")
      .toLocaleLowerCase()
      .split(/\s+/);
    const source = htmlAttribute(tag, "href");
    if (
      !relations.includes("icon") ||
      !source ||
      !shouldInlineReference(source)
    ) {
      return tag;
    }

    const assetPath = await resolveDistributionAsset(
      distributionDirectory,
      source,
    );
    return tag.replace(source, await assetDataUrl(assetPath));
  });
}

export async function buildV2Preview({
  distributionDirectory = defaultDistributionDirectory,
  outputPath = defaultOutputPath,
} = {}) {
  await assertOutputOutsideDistribution(distributionDirectory, outputPath);

  const distributionIndexPath = path.join(distributionDirectory, "index.html");
  const distributionIndex = await readFile(distributionIndexPath, "utf8");
  const entries = parseDistributionEntryAssets(distributionIndex);

  const inlineResult = await inlineEntriesForStandalone(
    distributionIndex,
    entries,
    distributionDirectory,
  );
  let standaloneHtml = inlineResult.html;
  standaloneHtml = stripDistributionFileRedirect(standaloneHtml);
  standaloneHtml = addStandaloneBootstrap(standaloneHtml);
  standaloneHtml = await inlineHtmlImageLinks(
    standaloneHtml,
    distributionDirectory,
  );
  standaloneHtml = standaloneHtml.replace(/^[\t ]+(?=\r?$)/gm, "");

  await writeFile(outputPath, standaloneHtml, "utf8");

  return {
    outputPath,
    scriptEntryCount: entries.filter(
      (entry) =>
        entry.kind === "script" || entry.kind === "vite-bootstrap",
    ).length,
    stylesheetEntryCount: entries.reduce(
      (count, entry) =>
        count +
        (entry.kind === "stylesheet"
          ? 1
          : entry.kind === "vite-bootstrap"
            ? entry.stylesheetSources.length
            : 0),
      inlineResult.externalViteStylesheetEntryCount,
    ),
  };
}

const isDirectExecution =
  process.argv[1] !== undefined && path.resolve(process.argv[1]) === scriptPath;

if (isDirectExecution) {
  buildV2Preview()
    .then(({ outputPath }) => {
      console.log(`Standalone preview written to ${outputPath}`);
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
