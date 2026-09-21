import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(process.cwd());
const rendererRoot = path.join(repositoryRoot, "src");

const forbiddenFragments = [
  "真实配置回读",
  "真实回读",
  "乐观成功状态",
  "安装准备度",
  "桌面应用接线",
  "零写入预览",
  "计划身份",
  "后端事件序号",
  "真实 Change Plan",
  "真实 Change Job",
  "当前权威状态",
  "受控错误",
  "成功证据",
  "assignment 结果",
  "认证观察器",
  "全局登录布尔值",
  "回读认证状态",
  "可验证结果",
  "配置投影",
  "数据库基线",
  "设备基线",
  "托管写入",
  "补偿步骤",
] as const;

type CopyOccurrence = {
  readonly file: string;
  readonly line: number;
  readonly text: string;
};

function listSourceFiles(directory: string): string[] {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) return listSourceFiles(entryPath);
      return /\.tsx?$/.test(entry.name) ? [entryPath] : [];
    })
    .sort();
}

function relativePath(file: string): string {
  return path.relative(rendererRoot, file).split(path.sep).join("/");
}

function collectCopy(file: string): CopyOccurrence[] {
  const sourceText = fs.readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(
    file,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const occurrences: CopyOccurrence[] = [];

  const add = (node: ts.Node, text: string) => {
    const normalized = text.replace(/\s+/g, " ").trim();
    if (!normalized) return;
    occurrences.push({
      file: relativePath(file),
      line:
        sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile))
          .line + 1,
      text: normalized,
    });
  };

  const visit = (node: ts.Node): void => {
    if (
      ts.isStringLiteralLike(node) ||
      ts.isNoSubstitutionTemplateLiteral(node) ||
      ts.isTemplateHead(node) ||
      ts.isTemplateMiddle(node) ||
      ts.isTemplateTail(node)
    ) {
      add(node, node.text);
    } else if (ts.isJsxText(node)) {
      add(node, node.getText(sourceFile));
    }
    ts.forEachChild(node, visit);
  };

  visit(sourceFile);
  return occurrences;
}

describe("FyAgent user-facing copy contract", () => {
  it("keeps reviewed secondary-page narration out of all route families", () => {
    // These are concrete retired strings, not an AI-authorship detector or
    // a length limit for useful explanations, warnings or user-authored text.
    const retiredCopy = [
      "正在读取已安装的 Skills",
      "正在读取该应用的提示词",
      "正在获取应用信息",
      "从左侧打开提示词后即可直接阅读和编辑正文。",
      "查看登录状态、软件连接和账号操作。",
      "查看账号连接、当前模型来源和需要处理的状态。",
      "登录状态与软件连接分别管理。",
      "当前搜索条件下没有结果",
      "并创建可恢复备份",
    ];
    const pagesRoot = path.join(rendererRoot, "pages");
    const files = listSourceFiles(pagesRoot);
    expect(
      [
        ...new Set(
          files.map(
            (file) => path.relative(pagesRoot, file).split(path.sep)[0],
          ),
        ),
      ].sort(),
    ).toEqual([
      "agents",
      "auth",
      "health",
      "mcp",
      "memory",
      "models",
      "projects",
      "prompts",
      "sessions",
      "skills",
    ]);
    const violations = files
      .flatMap(collectCopy)
      .filter((item) =>
        retiredCopy.some((fragment) => item.text.includes(fragment)),
      );
    expect(violations).toEqual([]);
  });

  it("does not expose reviewed implementation narration", () => {
    const violations = listSourceFiles(rendererRoot)
      .flatMap(collectCopy)
      .flatMap((occurrence) =>
        forbiddenFragments.flatMap((fragment) =>
          occurrence.text.includes(fragment)
            ? [
                `${occurrence.file}:${String(occurrence.line)} contains ${JSON.stringify(fragment)} in ${JSON.stringify(occurrence.text)}`,
              ]
            : [],
        ),
      );

    expect(
      violations,
      `Renderer exposed implementation-oriented copy:\n${violations.join("\n")}`,
    ).toEqual([]);
  });
});
