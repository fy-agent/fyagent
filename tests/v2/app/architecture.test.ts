import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(process.cwd());
const v2Root = path.join(repositoryRoot, "src", "v2");

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

function relativeV2Path(file: string): string {
  return path.relative(v2Root, file).split(path.sep).join("/");
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

const parsedModules = listSourceFiles(v2Root).map(parseModule);

const allowedLayerDependencies: Record<string, ReadonlySet<string>> = {
  root: new Set(["app"]),
  app: new Set(["app", "pages", "widgets", "shared", "dev"]),
  pages: new Set(["pages", "shared"]),
  widgets: new Set(["widgets", "shared"]),
  shared: new Set(["shared"]),
  dev: new Set(["dev", "shared"]),
};

describe("FyAgent V2 architecture boundary", () => {
  it("keeps every repository import inside src/v2", () => {
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) => {
        const target = resolveRepositoryImport(file, specifier);

        return target && !isWithin(v2Root, target)
          ? [`${relativeV2Path(file)}:${line} imports ${specifier}`]
          : [];
      }),
    );

    expect(
      violations,
      `V2 imported legacy renderer code:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("allows direct Tauri imports only in shared/platform/tauri", () => {
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) => {
        if (!specifier.startsWith("@tauri-apps/")) {
          return [];
        }

        return relativeV2Path(file).startsWith("shared/platform/tauri/")
          ? []
          : [`${relativeV2Path(file)}:${line} imports ${specifier}`];
      }),
    );

    expect(
      violations,
      `Direct Tauri imports escaped the adapter boundary:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps V2 dependencies within the declared layer direction", () => {
    const violations = parsedModules.flatMap(({ file, references }) => {
      const sourcePath = relativeV2Path(file);
      const sourceLayer = sourcePath.includes("/")
        ? sourcePath.split("/", 1)[0]
        : "root";
      const allowedTargets = allowedLayerDependencies[sourceLayer];

      return references.flatMap(({ line, specifier }) => {
        const target = resolveRepositoryImport(file, specifier);
        if (!target || !isWithin(v2Root, target)) {
          return [];
        }

        const [targetLayer] = relativeV2Path(target).split("/");
        return allowedTargets?.has(targetLayer)
          ? []
          : [`${sourcePath}:${line} imports ${specifier}`];
      });
    });

    expect(
      violations,
      `V2 layer direction was violated:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("rejects unsupported V2 UI dependency families", () => {
    const prohibitedPackages = new Set(["glasscn-ui", "lucide-react"]);
    const violations = parsedModules.flatMap(({ references }) =>
      references.flatMap(({ file, line, specifier }) =>
        prohibitedPackages.has(specifier.split("/", 1)[0])
          ? [`${relativeV2Path(file)}:${line} imports ${specifier}`]
          : [],
      ),
    );

    expect(
      violations,
      `V2 imported a prohibited UI package:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps import targets statically auditable", () => {
    const violations = parsedModules.flatMap(
      ({ file, nonLiteralDynamicImports }) =>
        nonLiteralDynamicImports.map(
          (line) =>
            `${relativeV2Path(file)}:${line} uses a non-literal dynamic import`,
        ),
    );

    expect(
      violations,
      `V2 contains imports that cannot be statically audited:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("does not create a second currentView state source", () => {
    const violations: string[] = [];

    for (const { file, sourceFile } of parsedModules) {
      const visit = (node: ts.Node): void => {
        if (ts.isIdentifier(node) && node.text === "currentView") {
          violations.push(
            `${relativeV2Path(file)}:${lineNumber(sourceFile, node)}`,
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
});
