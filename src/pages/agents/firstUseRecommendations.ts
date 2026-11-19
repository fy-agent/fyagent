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
  office: ["workbuddy"],
  coding: ["codex", "claude-code", "opencode", "omp", "grokbuild"],
  both: ["workbuddy", "codex"],
};

const reasons: Record<AgentCatalogId, string> = {
  qoderwork: "整理文件、处理数据与生成文档（仅支持官方模型）",
  "trae-work": "文档、演示稿与资料调研（仅支持官方模型）",
  workbuddy: "桌面办公全能助手，支持接入自有第三方模型与 Skills",
  grokbuild: "终端编码工具，支持接入自定义 Provider 与 MCP",
  codex: "支持桌面端与终端编码开发，可一键接入主流大模型",
  "claude-code": "在终端中理解、修改和测试代码，支持第三方兼容网关",
  opencode: "开源多端编码工具，支持同时连接多个自定义模型 Provider",
  omp: "轻量终端编码 Agent，支持通过 YAML 无损管理自有模型",
};

export function firstUseRecommendations(
  entries: readonly AgentCatalogEntry[],
  purpose: GuidePurpose,
) {
  const order = recommendedIds[purpose];
  return order.flatMap((id) => {
    const entry = entries.find((candidate) => candidate.id === id);
    if (!entry) return [];
    return [
      {
        entry,
        reason: reasons[entry.id],
        badge: "可接入自有模型",
      },
    ];
  });
}
