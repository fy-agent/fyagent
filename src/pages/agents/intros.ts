import type { AgentCatalogId } from "../../shared/features/types";

export type AgentIntro = {
  paragraphs: readonly string[];
};

const INTROS: Partial<Record<AgentCatalogId, AgentIntro>> = {
  qoderwork: {
    paragraphs: [
      "QoderWork CN 是 Qoder 的桌面工作助手，可处理文档、表格、演示文稿、研究、浏览器操作和其他本地任务。",
      "它可读取你授权的目录，并通过 Skill、MCP 和连接器扩展。开始任务前请检查工作目录；涉及文件或系统修改时，请确认操作范围。",
    ],
  },
  "trae-work": {
    paragraphs: [
      "TRAE Work CN 是 TRAE 的工作助手，支持文档、数据分析、深度研究、演示文稿和代码任务，可在桌面端和网页端使用。",
      "Work、Code、Design 三种模式分别面向办公、开发和设计任务。自定义模型需要在 TRAE Work CN 中添加；此处只读取当前可见的模型 ID。",
    ],
  },
  workbuddy: {
    paragraphs: [
      "WorkBuddy 是腾讯的桌面工作助手，可处理文档、数据、演示文稿、研究、文件整理和代码任务。",
      "它支持 Skill 扩展，也能读取本地文件和运行命令。模型设置可在模型管理中维护；执行可能修改本机内容的任务前，请检查工作目录和操作范围。",
    ],
  },
  grokbuild: {
    paragraphs: [
      "Grok Build 是 xAI 的终端编码工具，可阅读代码库、编辑文件、运行命令，并在全屏 TUI、无头模式或 Agent Client Protocol（ACP）中使用。",
      "它支持 Skills、插件、hooks 和 MCP。安装与登录状态可在此检查，Provider 和模型设置可在模型管理中维护。",
    ],
  },
  "claude-code": {
    paragraphs: [
      "Claude Code 是 Anthropic 的编码工具，可在终端、IDE、桌面应用和浏览器中使用。它能读取代码库、编辑文件、运行命令，并处理跨文件的开发任务。",
      "登录和安装由 Claude Code 官方入口完成。安装与登录状态可在此检查；第三方 Provider、模型、Skills、MCP 和提示词可在对应页面管理。",
    ],
  },
  opencode: {
    paragraphs: [
      "OpenCode 是开源编码工具，可在终端 TUI、桌面应用、网页和 IDE 中使用，也可通过本地 server 与 SDK 接入其他工具。",
      "OpenCode 分别连接各个模型 Provider。连接和模型可在此查看，相关配置可在对应页面维护。OpenCode 的权限规则控制文件与命令操作；需要进程隔离时，仍应使用容器或虚拟机。",
    ],
  },
};

export function getAgentIntro(id: AgentCatalogId): AgentIntro | null {
  return INTROS[id] ?? null;
}
