import type { AgentCatalogId } from "../../shared/features/types";

export type AgentIntro = {
  paragraphs: readonly string[];
};

const INTROS: Partial<Record<AgentCatalogId, AgentIntro>> = {
  qoderwork: {
    paragraphs: [
      "QoderWork CN 是阿里云 Qoder CN 系列里的桌面端智能工作助手。它把 Agent 能力从写代码扩展到日常办公：用自然语言交代任务，由它在本机整理文件、处理数据、撰写文档，并直接操作表格、演示文稿和 PDF。",
      "和只负责问答的聊天工具不同，QoderWork CN 强调把结果交付出来。它可以读取你授权的本地目录，自动浏览网页、填写表单、提取数据，也能点击、输入、滚动，与屏幕上的桌面应用协作。还可以设置一次性或周期性任务，并把钉钉、飞书等即时通讯接到工作流里。",
      "扩展方式包括社区技能、自定义 Skill、MCP 服务器和第三方连接器。文件处理在本地完成，访问目录前需要授权；高风险操作会再确认。它使用 Qoder CN 账号，并与系列内其他产品共享 Credits。",
    ],
  },
  "trae-work": {
    paragraphs: [
      "TRAE Work CN（TraeWork）是字节跳动推出的 AI 原生工作台，覆盖文档撰写、数据分析、深度调研、PPT 生成和代码开发。产品提供网页版、桌面版和移动端，桌面端不依赖 TraeCode IDE 也能独立运行。",
      "同一工作台里可以切换 Work、Code、Design 三种模式。Work 面向产品、运营和分析等岗位，处理文档、数据和演示稿；Code 面向开发，覆盖编码、调试、代码库和 Git 工作流；Design 用 AI 走完设计、修改到交付。你提出任务后，TraeWork 会自动拆解并调用 Skills 与工具，项目文件集中在 Workspace 里验收。",
      "桌面版同时支持本地与云端运行环境，网页版以云端为主；多任务可以并行。外部能力可通过 GitHub、飞书、MCP、技能、斜杠命令和规则扩展。自定义模型需要在 TRAE Work CN 自己的界面里添加。",
    ],
  },
  workbuddy: {
    paragraphs: [
      "WorkBuddy 是 workbuddy.cn 上的 AI 编程助手产品线（官方文档中也以 CodeBuddy 形态出现），由腾讯云提供，目标是把产品构思、设计、研发到部署放进同一条协作链路。",
      "常见形态包括 IDE、编辑器插件和 CLI。IDE 主打「对话即编程」，可以从自然语言需求生成结构化说明、把设计稿转成可维护代码，并覆盖补全、多文件改写、审查和测试。插件装进 VS Code 或 JetBrains，让开发者继续主导、AI 做辅助。CLI 面向命令行和自动化，适合脚本、运维和流水线里的批量任务。",
      "文档还提到需求分析转 PRD、Figma 转代码、CloudBase / Supabase 等云服务集成，以及混元、DeepSeek 等多模型切换。Ask / Craft / Plan 等模式用来区分问答、局部改写和跨文件执行。",
    ],
  },
  grokbuild: {
    paragraphs: [
      "Grok Build 是 xAI 的终端编码 Agent，命令行里通常以 grok 启动。它提供全屏 TUI，能阅读代码库、编辑文件、执行 shell、搜索网页，并管理较长时间的任务；也可以无头运行，或通过 Agent Client Protocol 嵌进其他编辑器。",
      "官方定位是覆盖计划、构建、测试到部署的开发工作流。当前由 Grok 4.6 驱动。安装脚本为 curl -fsSL https://x.ai/cli/install.sh | bash。首次启动会走浏览器登录。",
      "它可以按项目发现 rules、skills、plugins、hooks 和 MCP。子命令覆盖登录、模型列表、MCP 与插件市场、会话导入导出（含从 Claude Code 导入）、worktree 和仪表盘。headless / ACP 适合脚本和 CI。",
    ],
  },
  "claude-code": {
    paragraphs: [
      "Claude Code 是 Anthropic 的 agentic 编码工具，可以在终端、IDE、桌面应用和浏览器里使用。它直接读取代码库、编辑文件、运行命令，并接入 Git 与常见开发工具，用自然语言完成例行改动、解释复杂代码和提交工作流。",
      "终端 CLI 是完整功能面。官方安装不再推荐 npm：macOS 使用官方 install.sh 脚本，Windows 使用官方 PowerShell 安装命令。装好后在项目目录运行 claude 即可开会话。多数表面需要 Claude 订阅或 Anthropic Console 账号；终端和 VS Code 也支持第三方供应商。",
      "它能做代码库导览、多文件编辑、测试与 PR。除 CLI 外还有 VS Code / JetBrains 扩展、GitHub 集成、网页和移动入口。第三方网关应把 ANTHROPIC_BASE_URL 写成主机和前缀，不要再叠一层 /v1，否则 Anthropic SDK 拼接后会变成 /v1/v1/…。",
    ],
  },
  opencode: {
    paragraphs: [
      "OpenCode 是开源 AI 编码 Agent，可在终端 TUI、桌面应用和 IDE 扩展里使用。它把模型接到你的仓库、终端和开发工具：修复缺陷时可以自己找文件、拟定计划、改代码、跑测试并处理报错。",
      "官方强调隐私：不把你的代码或上下文存到他们那边，方便在对数据敏感的环境使用。它按 MIT 许可发布，模型无关，可用任意 LLM 供应商；模型费用与软件本身分开。推荐安装：curl -fsSL https://opencode.ai/install | bash。",
      "架构上是本地客户端加本地 server。TUI、桌面、IDE 和 SDK 都通过 HTTP 连这台 server，也支持 opencode attach 远程接入和 opencode serve 无头运行。权限提示用于确认写文件和跑命令，但官方说明它不是安全沙箱；需要隔离时应放进容器或虚拟机。",
    ],
  },
};

export function getAgentIntro(id: AgentCatalogId): AgentIntro | null {
  return INTROS[id] ?? null;
}
