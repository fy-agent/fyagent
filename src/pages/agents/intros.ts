import type { AgentCatalogId } from "../../shared/features/types";

export type AgentIntro = {
  paragraphs: readonly string[];
};

const INTROS: Partial<Record<AgentCatalogId, AgentIntro>> = {
  qoderwork: {
    paragraphs: [
      "Qoder 桌面工作助手，擅长文档、表格、研究与本地任务处理，支持 Skill、MCP 与连接器扩展。",
    ],
  },
  "trae-work": {
    paragraphs: [
      "TRAE 工作助手，覆盖办公（Work）、开发（Code）与设计（Design）三种模式，支持桌面端与网页端。",
    ],
  },
  workbuddy: {
    paragraphs: [
      "腾讯桌面工作助手，支持文档处理、数据分析、研究整理与代码任务，可通过 Skill 扩展能力。",
    ],
  },
  grokbuild: {
    paragraphs: [
      "xAI 终端编码工具，支持代码库阅读、文件编辑、命令执行及 Skills、插件与 MCP 扩展。",
    ],
  },
  "claude-code": {
    paragraphs: [
      "Anthropic 编码工具，可在终端、IDE 与桌面应用中使用，支持跨文件开发与 Skills、MCP 扩展。",
    ],
  },
  opencode: {
    paragraphs: [
      "开源编码工具，可在终端、桌面、网页与 IDE 中使用，支持接入多家模型服务商。",
    ],
  },
  omp: {
    paragraphs: [
      "Oh My Pi (omp) 轻量级终端编码 Agent，支持多服务商模型配置与自定义默认模型。",
    ],
  },
};

export function getAgentIntro(id: AgentCatalogId): AgentIntro | null {
  return INTROS[id] ?? null;
}
