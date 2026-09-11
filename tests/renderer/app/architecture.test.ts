import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(process.cwd());
const sourceRoot = path.join(repositoryRoot, "src");

interface ModuleReference {
  file: string;
  line: number;
  specifier: string;
}

interface ParsedModule {
  file: string;
  sourceFile: ts.SourceFile;
  references: ModuleReference[];
  nonLiteralDynamicImports: number[];
}

function listSourceFiles(directory: string): string[] {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const entryPath = path.join(directory, entry.name);

      if (entry.isDirectory()) {
        return listSourceFiles(entryPath);
      }

      return /\.tsx?$/.test(entry.name) ? [entryPath] : [];
    })
    .sort();
}

function lineNumber(sourceFile: ts.SourceFile, node: ts.Node): number {
  return (
    sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1
  );
}

function parseModule(file: string): ParsedModule {
  const sourceText = fs.readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(
    file,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const references: ModuleReference[] = [];
  const nonLiteralDynamicImports: number[] = [];

  const addReference = (node: ts.Node, specifier: ts.StringLiteralLike) => {
    references.push({
      file,
      line: lineNumber(sourceFile, node),
      specifier: specifier.text,
    });
  };

  const visit = (node: ts.Node): void => {
    if (
      (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) &&
      node.moduleSpecifier &&
      ts.isStringLiteralLike(node.moduleSpecifier)
    ) {
      addReference(node, node.moduleSpecifier);
    } else if (
      ts.isCallExpression(node) &&
      node.expression.kind === ts.SyntaxKind.ImportKeyword
    ) {
      const [argument] = node.arguments;
      if (argument && ts.isStringLiteralLike(argument)) {
        addReference(node, argument);
      } else {
        nonLiteralDynamicImports.push(lineNumber(sourceFile, node));
      }
    } else if (
      ts.isImportTypeNode(node) &&
      ts.isLiteralTypeNode(node.argument) &&
      ts.isStringLiteralLike(node.argument.literal)
    ) {
      addReference(node, node.argument.literal);
    }

    ts.forEachChild(node, visit);
  };

  visit(sourceFile);
  return { file, sourceFile, references, nonLiteralDynamicImports };
}

function relativeSourcePath(file: string): string {
  return path.relative(sourceRoot, file).split(path.sep).join("/");
}

function isWithin(parent: string, candidate: string): boolean {
  const relative = path.relative(parent, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
}

function isAllowedRepositoryTarget(target: string): boolean {
  if (!isWithin(sourceRoot, target)) return false;
  const [layer, ...parts] = relativeSourcePath(target).split("/");
  return (
    parts.length > 0 &&
    Object.prototype.hasOwnProperty.call(allowedLayerDependencies, layer)
  );
}

function resolveRepositoryImport(
  importer: string,
  specifier: string,
): string | undefined {
  const cleanSpecifier = specifier.split(/[?#]/, 1)[0];

  if (cleanSpecifier.startsWith(".")) {
    return path.resolve(path.dirname(importer), cleanSpecifier);
  }

  if (cleanSpecifier.startsWith("@/")) {
    return path.resolve(repositoryRoot, "src", cleanSpecifier.slice(2));
  }

  return undefined;
}

const parsedModules = listSourceFiles(sourceRoot)
  .filter((file) => !file.endsWith(".test.ts"))
  .map(parseModule);

const retiredWindowFrameIdentifiers = new Set([
  "WindowFramePort",
  "createWindowFramePort",
  "createBrowserWindowFramePort",
  "createTauriWindowFramePort",
  "windowFramePort",
  "prepareFrame",
]);

const allowedLayerDependencies: Record<string, ReadonlySet<string>> = {
  root: new Set(["app"]),
  app: new Set(["app", "pages", "widgets", "shared", "dev", "domain"]),
  pages: new Set(["pages", "shared", "domain"]),
  widgets: new Set(["widgets", "shared"]),
  shared: new Set(["shared", "domain"]),
  domain: new Set(["domain"]),
  dev: new Set(["dev", "shared"]),
};

describe("FyAgent single renderer architecture boundary", () => {
  it("requires an explicit origin contract at every production dialog and wrapper call", () => {
    const violations: string[] = [];
    let count = 0;
    for (const module of parsedModules) {
      if (relativeSourcePath(module.file).startsWith("dev/")) continue;
      const aliases = new Set<string>();
      for (const statement of module.sourceFile.statements) {
        if (!ts.isImportDeclaration(statement)) continue;
        const bindings = statement.importClause?.namedBindings;
        if (bindings && ts.isNamedImports(bindings))
          for (const specifier of bindings.elements)
            if (/Dialog$/.test((specifier.propertyName ?? specifier.name).text))
              aliases.add(specifier.name.text);
      }
      const visit = (node: ts.Node) => {
        if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
          const name = node.tagName.getText(module.sourceFile);
          if (/^[A-Z]\w*Dialog$|^Dialog$/.test(name) || aliases.has(name)) {
            count++;
            const hasOrigin = node.attributes.properties.some(
              (attribute) =>
                ts.isJsxAttribute(attribute) &&
                attribute.name.getText(module.sourceFile) === "originRef",
            );
            if (!hasOrigin)
              violations.push(
                `${relativeSourcePath(module.file)}:${lineNumber(module.sourceFile, node)} ${name}`,
              );
          }
        }
        ts.forEachChild(node, visit);
      };
      visit(module.sourceFile);
    }
    expect(count).toBeGreaterThan(30);
    expect(violations).toEqual([]);
  });
  it("really scans the current renderer and domain rather than an empty retired directory", () => {
    const files = parsedModules.map((module) =>
      relativeSourcePath(module.file),
    );
    expect(files.length).toBeGreaterThan(150);
    expect(files).toEqual(
      expect.arrayContaining([
        "main.tsx",
        "domain/codex-desktop/parsers.ts",
        "shared/platform/tauri/feature-ports/agents.ts",
      ]),
    );
  });

  it("keeps the domain independent of rendering, native APIs and notifications", () => {
    const violations = parsedModules
      .filter((module) => relativeSourcePath(module.file).startsWith("domain/"))
      .flatMap((module) =>
        module.references.filter((reference) =>
          /^(react(?:-dom)?(?:\/|$)|@tauri-apps\/|react-i18next|sonner)/.test(
            reference.specifier,
          ),
        ),
      );
    expect(violations).toEqual([]);
  });
  it("keeps repository imports inside declared production roles", () => {
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) => {
        const target = resolveRepositoryImport(file, specifier);

        return target && !isAllowedRepositoryTarget(target)
          ? [`${relativeSourcePath(file)}:${line} imports ${specifier}`]
          : [];
      }),
    );

    expect(
      violations,
      `Renderer imported an undeclared source role:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("rejects retired generation roots and undeclared role imports", () => {
    const importer = path.join(sourceRoot, "shared", "fixture.ts");
    const allowedSpecifiers = ["@/domain/codex-desktop", "@/shared/ui/Button"];
    const prohibitedSpecifiers = [
      "@/shared",
      "@/v2/shared/ui/Button",
      "@/v3/pages/agents/Page",
      "@/components/CodexDesktopInstaller",
      "@/hooks/useCodexDesktopInstaller",
      "@/lib/api/codex-desktop",
      "@/i18n",
    ];

    expect(
      allowedSpecifiers.map((specifier) => {
        const target = resolveRepositoryImport(importer, specifier);
        return target ? isAllowedRepositoryTarget(target) : false;
      }),
    ).toEqual(allowedSpecifiers.map(() => true));
    expect(
      prohibitedSpecifiers.map((specifier) => {
        const target = resolveRepositoryImport(importer, specifier);
        return target ? isAllowedRepositoryTarget(target) : false;
      }),
    ).toEqual(prohibitedSpecifiers.map(() => false));
  });

  it("allows direct Tauri imports only in shared/platform/tauri", () => {
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) => {
        if (!specifier.startsWith("@tauri-apps/")) {
          return [];
        }

        return relativeSourcePath(file).startsWith("shared/platform/tauri/")
          ? []
          : [`${relativeSourcePath(file)}:${line} imports ${specifier}`];
      }),
    );

    expect(
      violations,
      `Direct Tauri imports escaped the adapter boundary:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps production dependencies within the declared layer direction", () => {
    const violations = parsedModules.flatMap(({ file, references }) => {
      const sourcePath = relativeSourcePath(file);
      const sourceLayer = sourcePath.includes("/")
        ? sourcePath.split("/", 1)[0]
        : "root";
      const allowedTargets = allowedLayerDependencies[sourceLayer];

      return references.flatMap(({ line, specifier }) => {
        const target = resolveRepositoryImport(file, specifier);
        if (!target || !isWithin(sourceRoot, target)) {
          return [];
        }

        const [targetLayer] = relativeSourcePath(target).split("/");
        return allowedTargets?.has(targetLayer)
          ? []
          : [`${sourcePath}:${line} imports ${specifier}`];
      });
    });

    expect(
      violations,
      `Renderer layer direction was violated:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("rejects unsupported UI dependency families", () => {
    const prohibitedPackages = new Set(["glasscn-ui", "lucide-react"]);
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) =>
        prohibitedPackages.has(specifier.split("/", 1)[0])
          ? [`${relativeSourcePath(file)}:${line} imports ${specifier}`]
          : [],
      ),
    );

    expect(
      violations,
      `Renderer imported a prohibited UI package:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps the liquid-glass dependency behind its shared adapter", () => {
    const adapterPath = "shared/ui/GlassMaterial.tsx";
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) =>
        specifier === "@samasante/liquid-glass" &&
        relativeSourcePath(file) !== adapterPath
          ? [`${relativeSourcePath(file)}:${line} imports ${specifier}`]
          : [],
      ),
    );

    expect(
      violations,
      `The liquid-glass package escaped its shared adapter:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps framer-motion behind reviewed shared motion owners", () => {
    const allowedOwners = new Set(["shared/ui/motion.ts"]);
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) =>
        specifier === "framer-motion" &&
        !allowedOwners.has(relativeSourcePath(file))
          ? [`${relativeSourcePath(file)}:${line} imports ${specifier}`]
          : [],
      ),
    );

    expect(
      violations,
      `framer-motion escaped reviewed shared motion owners:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("does not use layoutId scale projection for the selection pill", () => {
    const source = fs.readFileSync(
      path.join(sourceRoot, "shared/ui/SelectionLens.tsx"),
      "utf8",
    );

    expect(source).not.toMatch(/\blayoutId\s*=/);
    expect(source).not.toMatch(/\bLayoutGroup\b/);
  });

  it("does not collapse the selection pill to the track origin", () => {
    const source = fs.readFileSync(
      path.join(sourceRoot, "shared/ui/SelectionLens.tsx"),
      "utf8",
    );

    expect(source).not.toMatch(
      /selectionLensCollapsedOrigin|layoutSettleFrameCount|width\.set\(0\)|height\.set\(0\)/,
    );
    expect(source).toContain("box.layoutChange");
    expect(source).not.toMatch(/left\.set\(inset\)/);
    expect(source).not.toMatch(/top\.set\(inset\)/);
  });

  it("reuses shared feature chrome instead of page-local copies", () => {
    const pages: Array<{
      relative: string;
      owners: string[];
      forbidden: string[];
    }> = [
      {
        relative: "pages/skills/Page.tsx",
        owners: [
          "FeatureTabs",
          "FeatureSearch",
          "FeatureList",
          "FeaturePagination",
          "AssignmentPanel",
          "InstallTargetDialog",
        ],
        forbidden: [
          'className="fy-feature-tab"',
          "function InstallTargetPicker",
          'className="fy-feature-check-grid"',
        ],
      },
      {
        relative: "pages/mcp/Page.tsx",
        owners: [
          "FeatureTabs",
          "FeatureSearch",
          "FeatureList",
          "AssignmentPanel",
        ],
        forbidden: [
          'className="fy-feature-tab"',
          'className="fy-feature-check-grid"',
        ],
      },
      {
        relative: "pages/mcp/Discovery.tsx",
        owners: ["FeatureSearch", "InstallTargetDialog"],
        forbidden: [
          "function InstallTargetPicker",
          'className="fy-feature-check-grid"',
        ],
      },
      {
        relative: "pages/mcp/InstallDialog.tsx",
        owners: ["AssignmentPanel", "InstallTargetDialog"],
        forbidden: ["function InstallTargetPicker"],
      },
      {
        relative: "pages/memory/Page.tsx",
        owners: ["FeatureTabs", "FeatureSearch", "FeatureList"],
        forbidden: ['className="fy-feature-tab"'],
      },
      {
        relative: "pages/prompts/Page.tsx",
        owners: ["FeatureSearch", "FeatureList"],
        forbidden: [],
      },
      {
        relative: "pages/models/modelChips.tsx",
        owners: ["FeatureSearch"],
        forbidden: [],
      },
    ];
    const violations = pages.flatMap(({ relative, owners, forbidden }) => {
      const source = fs.readFileSync(path.join(sourceRoot, relative), "utf8");
      const missing = owners.flatMap((owner) =>
        source.includes(
          `from "../../shared/${owner === "InstallTargetDialog" ? "features/controls" : "ui"}/${owner}"`,
        )
          ? []
          : [`${relative} does not import ${owner}`],
      );
      const handRolled = forbidden.flatMap((token) =>
        source.includes(token) ? [`${relative} still hand-rolls ${token}`] : [],
      );
      return [...missing, ...handRolled];
    });

    expect(
      violations,
      `Feature pages must reuse shared chrome:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps import targets statically auditable", () => {
    const violations = parsedModules.flatMap(
      ({ file, nonLiteralDynamicImports }) =>
        nonLiteralDynamicImports.map(
          (line) =>
            `${relativeSourcePath(file)}:${line} uses a non-literal dynamic import`,
        ),
    );

    expect(
      violations,
      `Renderer contains imports that cannot be statically audited:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("loads all eight primary product pages through literal dynamic imports", () => {
    const pages = fs.readFileSync(
      path.join(sourceRoot, "app/primaryPages.tsx"),
      "utf8",
    );
    const routeModules = [
      "agents",
      "health",
      "auth",
      "models",
      "skills",
      "mcp",
      "prompts",
      "memory",
    ];

    for (const route of routeModules) {
      expect(pages).toContain(`import("../pages/${route}/Page")`);
      expect(pages).not.toMatch(
        new RegExp(`^import[^\\n]+pages/${route}/Page`, "mu"),
      );
    }
  });

  it("keeps visited primary routes behind PersistentSurface without render-phase state", () => {
    const outlet = fs.readFileSync(
      path.join(sourceRoot, "app/PersistentPrimaryOutlet.tsx"),
      "utf8",
    );
    const topBar = fs.readFileSync(
      path.join(sourceRoot, "widgets/app-shell/TopBar.tsx"),
      "utf8",
    );

    expect(outlet).toContain("<Outlet />");
    expect(outlet).toContain("PersistentSurface");
    expect(outlet).toContain("useState");
    expect(outlet).not.toMatch(/\buseEffect\b/);
    expect(outlet).not.toMatch(/\bsetVisited\b/);
    expect(topBar).not.toContain("ToolCluster");
    expect(
      fs.existsSync(path.join(sourceRoot, "widgets/app-shell/ToolCluster.tsx")),
    ).toBe(false);
    expect(
      fs.readFileSync(path.join(sourceRoot, "main.tsx"), "utf8"),
    ).toContain("prefetchPrimaryRoutes");
  });

  it("does not create a second currentView state source", () => {
    const violations: string[] = [];

    for (const { file, sourceFile } of parsedModules) {
      const visit = (node: ts.Node): void => {
        if (ts.isIdentifier(node) && node.text === "currentView") {
          violations.push(
            `${relativeSourcePath(file)}:${lineNumber(sourceFile, node)}`,
          );
        }
        ts.forEachChild(node, visit);
      };

      visit(sourceFile);
    }

    expect(
      violations,
      `Router location must remain the sole navigation state source:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps native chrome under its existing native owner", () => {
    const violations: string[] = [];

    for (const { file, sourceFile } of parsedModules) {
      const visit = (node: ts.Node): void => {
        if (
          ts.isIdentifier(node) &&
          retiredWindowFrameIdentifiers.has(node.text)
        ) {
          violations.push(
            `${relativeSourcePath(file)}:${lineNumber(sourceFile, node)} uses ${node.text}`,
          );
        }

        if (
          ts.isCallExpression(node) &&
          ts.isPropertyAccessExpression(node.expression) &&
          node.expression.name.text === "setDecorations" &&
          node.arguments.length === 1 &&
          node.arguments[0].kind === ts.SyntaxKind.FalseKeyword
        ) {
          violations.push(
            `${relativeSourcePath(file)}:${lineNumber(sourceFile, node)} disables system decorations`,
          );
        }

        if (
          ts.isJsxAttribute(node) &&
          node.name.getText(sourceFile) === "data-tauri-drag-region" &&
          relativeSourcePath(file) !== "widgets/app-shell/TopBar.tsx"
        ) {
          violations.push(
            `${relativeSourcePath(file)}:${lineNumber(sourceFile, node)} declares a native drag region`,
          );
        }

        ts.forEachChild(node, visit);
      };

      visit(sourceFile);
    }

    expect(
      violations,
      `System-owned native chrome leaked into the renderer:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps HTTP(S) jumps behind ExternalLinkButton", () => {
    const allowed = new Set(["shared/features/provider.tsx"]);
    const violations: string[] = [];

    for (const { file, sourceFile } of parsedModules) {
      const relative = relativeSourcePath(file);
      if (allowed.has(relative)) continue;
      const visit = (node: ts.Node): void => {
        if (
          ts.isPropertyAccessExpression(node) &&
          node.name.text === "openExternal" &&
          ts.isPropertyAccessExpression(node.expression) &&
          node.expression.name.text === "settings"
        ) {
          violations.push(
            `${relative}:${lineNumber(sourceFile, node)} calls settings.openExternal`,
          );
        }
        if (
          ts.isPropertyAccessExpression(node) &&
          node.name.text === "open" &&
          ts.isIdentifier(node.expression) &&
          node.expression.text === "window"
        ) {
          violations.push(
            `${relative}:${lineNumber(sourceFile, node)} calls window.open`,
          );
        }
        ts.forEachChild(node, visit);
      };
      visit(sourceFile);
    }

    expect(
      violations,
      `HTTP(S) jumps must use ExternalLinkButton / useOpenExternal:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps feature contracts domain-owned behind a named compatibility facade", () => {
    const featuresRoot = path.join(sourceRoot, "shared", "features");
    const facade = fs.readFileSync(path.join(featuresRoot, "types.ts"), "utf8");
    const domainOwners = [
      "assignments",
      "skills",
      "mcp",
      "settings",
      "agents",
      "models",
      "prompts",
      "memory",
    ] as const;

    expect(facade).not.toMatch(/\bexport\s+\*/u);
    expect(facade).not.toMatch(
      /\bexport\s+(?:interface|const|function|class|enum)\b/u,
    );

    for (const owner of domainOwners) {
      const ownerPath = path.join(featuresRoot, `${owner}.ts`);
      expect(
        fs.existsSync(ownerPath),
        `${owner}.ts must remain a domain owner`,
      ).toBe(true);
      expect(facade).toContain(`from "./${owner}"`);
      expect(fs.readFileSync(ownerPath, "utf8")).not.toContain(
        'from "./types"',
      );
    }
  });
});
