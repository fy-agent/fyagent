import type {
  AgentCatalogEntry,
  AgentCatalogId,
} from "../../shared/features/types";

export type GuidePurpose = "office" | "coding" | "both";

export const GUIDE_PURPOSES: readonly {
  id: GuidePurpose;
  label: string;
  description: string;
}[] = [
  { id: "office", label: "日常办公", description: "文档、表格与资料整理" },
  { id: "coding", label: "编程开发", description: "写代码、修问题与测试" },
  { id: "both", label: "两者都用", description: "办公与开发都会用" },
];

const recommendedIds: Record<GuidePurpose, readonly AgentCatalogId[]> = {
  office: ["qoderwork", "trae-work", "workbuddy"],
  coding: ["grokbuild", "codex", "claude-code", "opencode"],
  both: ["workbuddy", "codex"],
};

const reasons: Partial<Record<AgentCatalogId, string>> = {
  qoderwork: "整理文件、处理数据与生成文档",
  "trae-work": "文档、演示稿与资料调研",
  workbuddy: "处理日常办公任务",
  grokbuild: "在终端中编写代码与运行测试",
  codex: "开发功能、修复问题与检查代码",
  "claude-code": "在终端中理解、修改和测试代码",
  opencode: "在桌面或终端中编写代码",
};

export function firstUseRecommendations(
  entries: readonly AgentCatalogEntry[],
  purpose: GuidePurpose,
) {
  return entries
    .filter((entry) => recommendedIds[purpose].includes(entry.id))
    .map((entry) => ({
      entry,
      reason: reasons[entry.id] ?? entry.description,
    }));
}
