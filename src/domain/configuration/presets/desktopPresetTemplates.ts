import type { ApiProtocol } from "../providerApi";
import type {
  BatchSetupTargetId,
  BatchSetupTargetSpec,
} from "../../../shared/features/models";

export interface DesktopPresetTargetMapping {
  baseUrl: string;
  modelId: string;
  extraModelIds?: readonly string[];
  protocol: ApiProtocol;
}

export interface DesktopPresetTemplate {
  id: string;
  name: string;
  summary: string;
  apiKeyUrl: string;
  targets: Partial<Record<BatchSetupTargetId, DesktopPresetTargetMapping>>;
}

export const BATCH_CONFIGURABLE_TARGETS: readonly {
  id: BatchSetupTargetId;
  label: string;
  defaultChecked: boolean;
}[] = [
  { id: "workbuddy", label: "WorkBuddy", defaultChecked: true },
  { id: "codex", label: "Codex", defaultChecked: true },
  { id: "claude", label: "Claude Code", defaultChecked: true },
  { id: "opencode", label: "OpenCode", defaultChecked: true },
  { id: "omp", label: "Oh My Pi (omp)", defaultChecked: true },
  { id: "grokbuild", label: "Grok Build", defaultChecked: false },
];

export const DESKTOP_PRESET_TEMPLATES: readonly DesktopPresetTemplate[] = [
  {
    id: "deepseek",
    name: "DeepSeek",
    summary: "原生支持 Anthropic 与 OpenAI 协议，兼顾主力编码与快速推理",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    targets: {
      workbuddy: {
        baseUrl: "https://api.deepseek.com/v1",
        modelId: "deepseek-v4-pro",
        extraModelIds: ["deepseek-flash"],
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://api.deepseek.com",
        modelId: "deepseek-flash",
        protocol: "responses",
      },
      claude: {
        baseUrl: "https://api.deepseek.com/anthropic",
        modelId: "deepseek-v4-pro",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://api.deepseek.com/v1",
        modelId: "deepseek-v4-pro",
        extraModelIds: ["deepseek-flash"],
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://api.deepseek.com/v1",
        modelId: "deepseek-v4-pro",
        extraModelIds: ["deepseek-flash"],
        protocol: "chat",
      },
      grokbuild: {
        baseUrl: "https://api.deepseek.com",
        modelId: "deepseek-flash",
        protocol: "responses",
      },
    },
  },
  {
    id: "kimi",
    name: "Kimi (月之暗面)",
    summary: "Kimi K2.7 Code 编码模型，全平台兼容直连",
    apiKeyUrl: "https://platform.kimi.com/console/api-keys",
    targets: {
      workbuddy: {
        baseUrl: "https://api.moonshot.cn/v1",
        modelId: "kimi-k2.7-code",
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://api.moonshot.cn/v1",
        modelId: "kimi-k2.7-code",
        protocol: "responses",
      },
      claude: {
        baseUrl: "https://api.moonshot.cn/anthropic",
        modelId: "kimi-k2.7-code",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://api.moonshot.cn/v1",
        modelId: "kimi-k2.7-code",
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://api.moonshot.cn/v1",
        modelId: "kimi-k2.7-code",
        protocol: "chat",
      },
      grokbuild: {
        baseUrl: "https://api.moonshot.cn/v1",
        modelId: "kimi-k2.7-code",
        protocol: "responses",
      },
    },
  },
  {
    id: "zhipu",
    name: "智谱 GLM (BigModel)",
    summary: "自动按客户端协议匹配 /api/anthropic、/api/v1 与 /api/paas/v4",
    apiKeyUrl: "https://www.bigmodel.cn/claude-code",
    targets: {
      workbuddy: {
        baseUrl: "https://open.bigmodel.cn/api/paas/v4",
        modelId: "glm-5.3",
        extraModelIds: ["glm-5.3-flash"],
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://open.bigmodel.cn/api/v1",
        modelId: "glm-5.3",
        protocol: "responses",
      },
      claude: {
        baseUrl: "https://open.bigmodel.cn/api/anthropic",
        modelId: "glm-5.3",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://open.bigmodel.cn/api/coding/paas/v4",
        modelId: "glm-5.3",
        extraModelIds: ["glm-5.3-flash"],
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://open.bigmodel.cn/api/paas/v4",
        modelId: "glm-5.3",
        extraModelIds: ["glm-5.3-flash"],
        protocol: "chat",
      },
      grokbuild: {
        baseUrl: "https://open.bigmodel.cn/api/v1",
        modelId: "glm-5.3",
        protocol: "responses",
      },
    },
  },
  {
    id: "dashscope",
    name: "阿里云百炼 (通义千问)",
    summary: "Qwen 3.8 系列旗舰与极速模型，支持办公与研发全场景",
    apiKeyUrl: "https://bailian.console.aliyun.com/",
    targets: {
      workbuddy: {
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        modelId: "qwen3.8-max",
        extraModelIds: ["qwen3.8-flash"],
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        modelId: "qwen3.8-max",
        protocol: "chat",
      },
      claude: {
        baseUrl: "https://dashscope.aliyuncs.com/apps/anthropic",
        modelId: "qwen3.8-max",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        modelId: "qwen3.8-max",
        extraModelIds: ["qwen3.8-flash"],
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        modelId: "qwen3.8-max",
        extraModelIds: ["qwen3.8-flash"],
        protocol: "chat",
      },
    },
  },
  {
    id: "volcengine",
    name: "火山方舟 (豆包)",
    summary: "Doubao Seed Pro 模型，原生支持 /api/v3 Responses 与兼容接口",
    apiKeyUrl:
      "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    targets: {
      workbuddy: {
        baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
        modelId: "doubao-seed-2-1-pro-260628",
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
        modelId: "doubao-seed-2-1-pro-260628",
        protocol: "responses",
      },
      claude: {
        baseUrl: "https://ark.cn-beijing.volces.com/api/compatible",
        modelId: "doubao-seed-2-1-pro-260628",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
        modelId: "doubao-seed-2-1-pro-260628",
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
        modelId: "doubao-seed-2-1-pro-260628",
        protocol: "chat",
      },
      grokbuild: {
        baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
        modelId: "doubao-seed-2-1-pro-260628",
        protocol: "responses",
      },
    },
  },
  {
    id: "minimax",
    name: "MiniMax",
    summary: "MiniMax-M3 超长上下文模型，支持原生 Responses 与 Anthropic 接口",
    apiKeyUrl: "https://platform.minimax.cn/subscribe/token-plan",
    targets: {
      workbuddy: {
        baseUrl: "https://api.minimax.cn/v1",
        modelId: "MiniMax-M3",
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://api.minimax.cn/v1",
        modelId: "MiniMax-M3",
        protocol: "responses",
      },
      claude: {
        baseUrl: "https://api.minimax.cn/anthropic",
        modelId: "MiniMax-M3[1M]",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://api.minimax.cn/v1",
        modelId: "MiniMax-M3",
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://api.minimax.cn/v1",
        modelId: "MiniMax-M3",
        protocol: "chat",
      },
      grokbuild: {
        baseUrl: "https://api.minimax.cn/v1",
        modelId: "MiniMax-M3",
        protocol: "responses",
      },
    },
  },
  {
    id: "siliconflow",
    name: "硅基流动 (SiliconFlow)",
    summary: "聚合主流开源旗舰模型，一个 Key 调用多厂商推理能力",
    apiKeyUrl: "https://cloud.siliconflow.cn/",
    targets: {
      workbuddy: {
        baseUrl: "https://api.siliconflow.cn/v1",
        modelId: "deepseek-ai/DeepSeek-V4-Flash",
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://api.siliconflow.cn/v1",
        modelId: "deepseek-ai/DeepSeek-V4-Flash",
        protocol: "chat",
      },
      claude: {
        baseUrl: "https://api.siliconflow.cn",
        modelId: "Pro/MiniMaxAI/MiniMax-M2.5",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://api.siliconflow.cn/v1",
        modelId: "deepseek-ai/DeepSeek-V4-Flash",
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://api.siliconflow.cn/v1",
        modelId: "deepseek-ai/DeepSeek-V4-Flash",
        protocol: "chat",
      },
    },
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    summary: "全球主流大模型统一聚合路由，支持 Claude / GPT / Gemini 等",
    apiKeyUrl: "https://openrouter.ai/keys",
    targets: {
      workbuddy: {
        baseUrl: "https://openrouter.ai/api/v1",
        modelId: "anthropic/claude-sonnet-5",
        extraModelIds: ["gpt-5.6-sol"],
        protocol: "chat",
      },
      codex: {
        baseUrl: "https://openrouter.ai/api/v1",
        modelId: "gpt-5.6-sol",
        protocol: "responses",
      },
      claude: {
        baseUrl: "https://openrouter.ai/api",
        modelId: "anthropic/claude-sonnet-5",
        protocol: "anthropic",
      },
      opencode: {
        baseUrl: "https://openrouter.ai/api/v1",
        modelId: "anthropic/claude-sonnet-5",
        extraModelIds: ["gpt-5.6-sol"],
        protocol: "chat",
      },
      omp: {
        baseUrl: "https://openrouter.ai/api/v1",
        modelId: "anthropic/claude-sonnet-5",
        extraModelIds: ["gpt-5.6-sol"],
        protocol: "chat",
      },
      grokbuild: {
        baseUrl: "https://openrouter.ai/api/v1",
        modelId: "gpt-5.6-sol",
        protocol: "responses",
      },
    },
  },
];

export function buildBatchTargetSpecs(
  preset: DesktopPresetTemplate,
  selectedTargets: readonly BatchSetupTargetId[],
): BatchSetupTargetSpec[] {
  const specs: BatchSetupTargetSpec[] = [];
  for (const target of selectedTargets) {
    const mapping = preset.targets[target];
    if (!mapping) continue;
    specs.push({
      target,
      providerId: preset.id,
      providerName: preset.name,
      baseUrl: mapping.baseUrl,
      modelId: mapping.modelId,
      extraModelIds: mapping.extraModelIds ? [...mapping.extraModelIds] : [],
      protocol: mapping.protocol,
    });
  }
  return specs;
}
