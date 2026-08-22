import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(process.cwd());
const srcRoot = path.join(repositoryRoot, "src");
const sharedRoot = path.join(srcRoot, "shared");

interface ModuleReference {
  file: string;
  line: number;
  specifier: string;
}

function listSourceFiles(directory: string): string[] {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) return listSourceFiles(entryPath);
      return /\.tsx?$/u.test(entry.name) ? [entryPath] : [];
    })
    .sort();
}

function lineNumber(sourceFile: ts.SourceFile, node: ts.Node): number {
  return (
    sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1
  );
}

function parseReferences(file: string): ModuleReference[] {
  const source = fs.readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(
    file,
    source,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const references: ModuleReference[] = [];

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
      if (argument && ts.isStringLiteralLike(argument))
        addReference(node, argument);
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
  return references;
}

function repositoryRelative(file: string): string {
  return path.relative(repositoryRoot, file).split(path.sep).join("/");
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
  const cleanSpecifier = specifier.split(/[?#]/u, 1)[0];
  if (cleanSpecifier.startsWith(".")) {
    return path.resolve(path.dirname(importer), cleanSpecifier);
  }
  if (cleanSpecifier.startsWith("@/")) {
    return path.resolve(srcRoot, cleanSpecifier.slice(2));
  }
  return undefined;
}

describe("renderer architecture boundaries", () => {
  it("keeps src/shared renderer-generation-neutral", () => {
    const violations = listSourceFiles(sharedRoot).flatMap((file) =>
      parseReferences(file).flatMap(({ line, specifier }) => {
        const target = resolveRepositoryImport(file, specifier);
        if (target && !isWithin(sharedRoot, target)) {
          return [`${repositoryRelative(file)}:${line} imports ${specifier}`];
        }
        if (
          specifier === "react" ||
          specifier.startsWith("react/") ||
          specifier.startsWith("@tauri-apps/")
        ) {
          return [`${repositoryRelative(file)}:${line} imports ${specifier}`];
        }
        return [];
      }),
    );

    expect(
      violations,
      `Renderer-neutral shared code leaked into a renderer generation or platform runtime:\n${violations.join("\n")}`,
    ).toEqual([]);
  });

  it("keeps raw Tauri core access out of leftover feature code", () => {
    const allowedCoreImporters = new Set([
      "src/App.tsx",
      "src/components/DatabaseUpgrade.tsx",
      "src/components/theme-provider.tsx",
      "src/main.tsx",
    ]);
    const violations = listSourceFiles(srcRoot).flatMap((file) => {
      const relative = repositoryRelative(file);
      if (relative.startsWith("src/v2/") || relative.startsWith("src/lib/")) {
        return [];
      }

      return parseReferences(file).flatMap(({ line, specifier }) =>
        specifier === "@tauri-apps/api/core" &&
        !allowedCoreImporters.has(relative)
          ? [`${relative}:${line} imports ${specifier}`]
          : [],
      );
    });

    expect(
      violations,
      `Raw Tauri core access escaped the reviewed leftover adapters/bootstrap boundaries:\n${violations.join("\n")}`,
    ).toEqual([]);
  });
});
