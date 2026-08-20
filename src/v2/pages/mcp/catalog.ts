import { UserFacingError } from "../../shared/features/helpers";
import {
  buildNpxCommand,
  type McpLaunchPlatform,
} from "../../shared/features/mcpLaunch";
import { mcpRecipeIdentity } from "../../shared/features/mcpSecurity";
import {
  createMcpAssignments,
  type McpServer,
  type McpServerSpec,
  type McpTargetId,
} from "../../shared/features/types";

export type McpCatalogCategory =
  | "china"
  | "devtools"
  | "collab"
  | "maps"
  | "multimodal"
  | "basics"
  | "cloud";

export type McpCatalogFilterId = "all" | "ready" | "configure";

export type McpProviderGroup =
  | "alibaba"
  | "tencent"
  | "baidu"
  | "china-other"
  | "general";

export type McpCatalogMaturity = "stable" | "verify" | "compat" | "advanced";

export type McpPrivilege = "read" | "write" | "cloud";

export type McpProvenance = "official" | "reference" | "community";

export type McpInstallFieldType =
  | "text"
  | "password"
  | "path"
  | "select"
  | "multi-select";

export interface McpInstallFieldOption {
  value: string;
  label: string;
}

export interface McpInstallField {
  key: string;
  label: string;
  type: McpInstallFieldType;
  required?: boolean;
  placeholder?: string;
  help?: string;
  options?: readonly McpInstallFieldOption[];
}

export type McpInstallValues = Record<string, string | string[]>;

export interface McpCatalogItem {
  id: string;
  name: string;
  description: string;
  categories: readonly McpCatalogCategory[];
  tags: readonly string[];
  publisher: string;
  providerGroup: McpProviderGroup;
  provenance: McpProvenance;
  homepage?: string;
  docs?: string;
  requirements: readonly ("none" | "node" | "uv")[];
  fields: readonly McpInstallField[];
  authLabel: string;
  risk?: string;
  privilege?: McpPrivilege;
  maturity: McpCatalogMaturity;
  installable: boolean;
  disabledReason?: string;
  recommended?: boolean;
  build(
    values: McpInstallValues,
    apps: readonly McpTargetId[],
    platform: McpLaunchPlatform,
  ): McpServer;
}

export const MCP_CATEGORY_LABEL: Record<McpCatalogCategory, string> = {
  china: "国内服务",
  devtools: "开发工具",
  collab: "办公协作",
  maps: "地图生活",
  multimodal: "AI/多模态",
  basics: "基础能力",
  cloud: "云与数据",
};

export const MCP_CATALOG_FILTERS: ReadonlyArray<{
  id: McpCatalogFilterId;
  label: string;
}> = [
  { id: "all", label: "全部" },
  { id: "ready", label: "直接安装" },
  { id: "configure", label: "配置安装" },
];

export const MCP_CATALOG_PROVIDERS: ReadonlyArray<{
  id: "all" | McpProviderGroup;
  label: string;
}> = [
  { id: "all", label: "全部提供方" },
  { id: "alibaba", label: "阿里系" },
  { id: "tencent", label: "腾讯系" },
  { id: "baidu", label: "百度系" },
  { id: "china-other", label: "其他国内" },
  { id: "general", label: "通用" },
];

export const MCP_PRIVILEGE_LABEL: Record<McpPrivilege, string> = {
  read: "只读/查询",
  write: "可写数据",
  cloud: "可操作云资源",
};

export const MCP_MATURITY_LABEL: Record<McpCatalogMaturity, string | null> = {
  stable: null,
  verify: "待验证",
  compat: "等待 Streamable HTTP",
  advanced: "高权限未开放",
};

const DINGTALK_PROFILES: readonly McpInstallFieldOption[] = [
  { value: "chatbot", label: "机器人" },
  { value: "calendar", label: "日历" },
  { value: "contact", label: "通讯录" },
  { value: "todo", label: "待办" },
];

const YUNXIAO_TOOLSETS: readonly McpInstallFieldOption[] = [
  { value: "codeup", label: "Codeup 代码" },
  { value: "projex", label: "Projex 项目" },
  { value: "flow", label: "流水线" },
  { value: "packages", label: "制品" },
];

const GITEE_READONLY_TOOLS = [
  "list_user_repos",
  "get_file_content",
  "list_releases",
  "search_open_source_repositories",
  "list_repo_pulls",
  "get_pull_detail",
  "list_pull_comments",
  "get_diff_files",
  "get_repo_issue_detail",
  "list_repo_issues",
  "list_issue_comments",
  "get_user_info",
  "search_users",
  "list_user_notifications",
].join(",");

function requiredText(
  values: McpInstallValues,
  key: string,
  label: string,
): string {
  const value = values[key];
  const text = typeof value === "string" ? value.trim() : "";
  if (!text) throw new UserFacingError(`请填写${label}`);
  return text;
}

function optionalText(values: McpInstallValues, key: string): string {
  const value = values[key];
  return typeof value === "string" ? value.trim() : "";
}

function selectedList(values: McpInstallValues, key: string): string[] {
  const value = values[key];
  if (Array.isArray(value)) {
    return value.map((item) => item.trim()).filter(Boolean);
  }
  return String(value ?? "")
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function assertRequiredFields(
  fields: readonly McpInstallField[],
  values: McpInstallValues,
): void {
  for (const field of fields) {
    if (!field.required) continue;
    if (field.type === "multi-select" || field.type === "path") {
      if (selectedList(values, field.key).length === 0) {
        throw new UserFacingError(`请填写${field.label}`);
      }
      continue;
    }
    requiredText(values, field.key, field.label);
  }
}

function catalogItem(
  config: Omit<McpCatalogItem, "build" | "installable" | "maturity"> & {
    installable?: boolean;
    maturity?: McpCatalogMaturity;
    buildSpec: (
      values: McpInstallValues,
      platform: McpLaunchPlatform,
    ) => McpServerSpec;
  },
): McpCatalogItem {
  const {
    buildSpec,
    installable = true,
    maturity = "stable",
    ...item
  } = config;
  return {
    ...item,
    installable,
    maturity,
    build(values, apps, platform) {
      if (!installable) {
        throw new UserFacingError(item.disabledReason ?? "暂未开放安装");
      }
      if (apps.length === 0) {
        throw new UserFacingError("请选择至少一个 Agent");
      }
      assertRequiredFields(item.fields, values);
      return {
        id: item.id,
        name: item.name,
        description: item.description,
        tags: [...item.tags],
        homepage: item.homepage,
        docs: item.docs,
        apps: createMcpAssignments(apps),
        server: buildSpec(values, platform),
      };
    },
  };
}

function npxSpec(
  packageName: string,
  platform: McpLaunchPlatform,
  extra: {
    extraArgs?: readonly string[];
    env?: Record<string, string>;
  } = {},
): McpServerSpec {
  const launch = buildNpxCommand(packageName, extra.extraArgs ?? [], platform);
  return {
    type: "stdio",
    ...launch,
    ...(extra.env ? { env: extra.env } : {}),
  };
}

export const MCP_CATALOG: readonly McpCatalogItem[] = [
  catalogItem({
    id: "amap",
    name: "高德地图 MCP",
    description: "地点搜索、路线规划、天气与地理编码。",
    categories: ["china", "maps"],
    tags: ["地图", "出行", "HTTP"],
    publisher: "高德开放平台",
    providerGroup: "china-other",
    provenance: "official",
    homepage: "https://lbs.amap.com/api/mcp-server/summary",
    docs: "https://lbs.amap.com/api/mcp-server/summary",
    requirements: ["none"],
    authLabel: "API Key",
    privilege: "read",
    recommended: true,
    fields: [
      {
        key: "key",
        label: "API Key",
        type: "password",
        required: true,
        help: "Key 仅用于生成 MCP 配置；普通详情与搜索会脱敏。",
      },
    ],
    buildSpec: (values) => ({
      type: "http",
      url: `https://mcp.amap.com/mcp?key=${requiredText(values, "key", "API Key")}`,
    }),
  }),
  catalogItem({
    id: "baidu-map",
    name: "百度地图 MCP",
    description: "地点检索、路线规划与地理编码。",
    categories: ["china", "maps"],
    tags: ["地图", "出行", "stdio"],
    publisher: "百度地图开放平台",
    providerGroup: "baidu",
    provenance: "official",
    homepage: "https://lbsyun.baidu.com/faq/api?title=mcp/introduce",
    docs: "https://lbsyun.baidu.com/faq/api?title=mcp/introduce",
    requirements: ["node"],
    authLabel: "API Key",
    privilege: "read",
    fields: [
      {
        key: "apiKey",
        label: "百度地图 API Key",
        type: "password",
        required: true,
      },
    ],
    buildSpec: (values, platform) =>
      npxSpec("@baidumap/mcp-server-baidu-map", platform, {
        env: {
          BAIDU_MAP_API_KEY: requiredText(values, "apiKey", "百度地图 API Key"),
        },
      }),
  }),
  catalogItem({
    id: "feishu",
    name: "飞书 OpenAPI MCP",
    description: "文档、消息、日历等企业协作能力。",
    categories: ["china", "collab"],
    tags: ["飞书", "办公", "stdio"],
    publisher: "飞书开放平台",
    providerGroup: "china-other",
    provenance: "official",
    homepage:
      "https://open.feishu.cn/document/uAjLw4CM/ukTMukTMukTM/mcp_integration/mcp_introduction",
    docs: "https://open.feishu.cn/document/uAjLw4CM/ukTMukTMukTM/mcp_integration/mcp_introduction",
    requirements: ["node"],
    authLabel: "App ID + Secret",
    privilege: "write",
    recommended: true,
    risk: "可访问企业文档、消息与日历等数据。",
    fields: [
      { key: "appId", label: "App ID", type: "text", required: true },
      {
        key: "appSecret",
        label: "App Secret",
        type: "password",
        required: true,
      },
    ],
    buildSpec: (values, platform) =>
      npxSpec("@larksuiteoapi/lark-mcp", platform, {
        extraArgs: [
          "mcp",
          "-a",
          requiredText(values, "appId", "App ID"),
          "-s",
          requiredText(values, "appSecret", "App Secret"),
        ],
      }),
  }),
  catalogItem({
    id: "dingtalk",
    name: "钉钉 MCP",
    description: "通讯录、日历、机器人与待办等企业协作能力。",
    categories: ["china", "collab"],
    tags: ["钉钉", "办公", "stdio"],
    publisher: "钉钉开放平台",
    providerGroup: "china-other",
    provenance: "official",
    homepage: "https://open.dingtalk.com/document/orgapp/mcp-server",
    docs: "https://open.dingtalk.com/document/orgapp/mcp-server",
    requirements: ["node"],
    authLabel: "Client ID + Secret",
    privilege: "write",
    risk: "可访问企业通讯录、日程与待办等数据。",
    fields: [
      { key: "clientId", label: "Client ID", type: "text", required: true },
      {
        key: "clientSecret",
        label: "Client Secret",
        type: "password",
        required: true,
      },
      {
        key: "profiles",
        label: "能力 Profiles",
        type: "multi-select",
        required: true,
        help: "按需开启，不要一次授予全部能力。",
        options: DINGTALK_PROFILES,
      },
    ],
    buildSpec: (values, platform) => {
      const profiles = selectedList(values, "profiles");
      if (profiles.includes("ALL")) {
        throw new UserFacingError("请按需选择钉钉能力，不要使用全部授权。");
      }
      return npxSpec("dingtalk-mcp@latest", platform, {
        env: {
          DINGTALK_Client_ID: requiredText(values, "clientId", "Client ID"),
          DINGTALK_Client_Secret: requiredText(
            values,
            "clientSecret",
            "Client Secret",
          ),
          ACTIVE_PROFILES: profiles.join(","),
        },
      });
    },
  }),
  catalogItem({
    id: "yunxiao",
    name: "云效 DevOps MCP",
    description: "阿里云效代码、项目与流水线协作。",
    categories: ["china", "devtools"],
    tags: ["云效", "DevOps", "HTTP"],
    publisher: "阿里云云效",
    providerGroup: "alibaba",
    provenance: "official",
    homepage:
      "https://help.aliyun.com/zh/yunxiao/developer-reference/use-the-alibaba-cloud-devops-mcp-server",
    docs: "https://help.aliyun.com/zh/yunxiao/developer-reference/use-the-alibaba-cloud-devops-mcp-server",
    requirements: ["none"],
    authLabel: "Access Token",
    privilege: "write",
    risk: "部分工具具有写操作，请按需限制 toolsets。",
    fields: [
      {
        key: "token",
        label: "Personal Access Token",
        type: "password",
        required: true,
      },
      {
        key: "toolsets",
        label: "Toolsets",
        type: "multi-select",
        options: YUNXIAO_TOOLSETS,
        help: "留空则使用远端默认能力集合。",
      },
    ],
    buildSpec: (values) => {
      const toolsets = selectedList(values, "toolsets");
      const url = toolsets.length
        ? `https://openapi-rdc.aliyuncs.com/ai/mcp?toolsets=${encodeURIComponent(toolsets.join(","))}`
        : "https://openapi-rdc.aliyuncs.com/ai/mcp";
      return {
        type: "http",
        url,
        headers: {
          Authorization: `Bearer ${requiredText(values, "token", "Personal Access Token")}`,
        },
      };
    },
  }),
  catalogItem({
    id: "context7",
    name: "Context7",
    description: "按库检索最新文档，辅助编码 Agent 引用正确 API。",
    categories: ["devtools"],
    tags: ["文档", "检索", "HTTP"],
    publisher: "Context7",
    providerGroup: "general",
    provenance: "official",
    homepage: "https://context7.com",
    docs: "https://github.com/upstash/context7",
    requirements: ["none"],
    authLabel: "API Key（可选）",
    privilege: "read",
    fields: [
      {
        key: "apiKey",
        label: "API Key",
        type: "password",
        help: "推荐填写；留空也可先注册远程连接。",
      },
    ],
    buildSpec: (values) => {
      const apiKey =
        typeof values.apiKey === "string" ? values.apiKey.trim() : "";
      return {
        type: "http",
        url: "https://mcp.context7.com/mcp",
        ...(apiKey ? { headers: { Authorization: `Bearer ${apiKey}` } } : {}),
      };
    },
  }),
  catalogItem({
    id: "playwright",
    name: "Playwright MCP",
    description: "浏览器自动化与页面探索。",
    categories: ["devtools"],
    tags: ["浏览器", "自动化", "stdio"],
    publisher: "Microsoft",
    providerGroup: "general",
    provenance: "official",
    homepage: "https://github.com/microsoft/playwright-mcp",
    docs: "https://github.com/microsoft/playwright-mcp",
    requirements: ["node"],
    authLabel: "无",
    privilege: "write",
    risk: "可访问网页、会话状态并执行页面操作；MCP 本身不是安全边界。",
    fields: [],
    buildSpec: (_values, platform) =>
      npxSpec("@playwright/mcp@latest", platform),
  }),
  catalogItem({
    id: "filesystem",
    name: "Filesystem",
    description: "按白名单目录读写本地文件。",
    categories: ["basics"],
    tags: ["文件", "本地", "stdio"],
    publisher: "Model Context Protocol",
    providerGroup: "general",
    provenance: "reference",
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem",
    requirements: ["node"],
    authLabel: "目录白名单",
    privilege: "write",
    risk: "本地文件高权限。必须指定允许目录，空目录不等于全盘。",
    fields: [
      {
        key: "paths",
        label: "允许目录",
        type: "path",
        required: true,
        placeholder: "每行一个目录",
        help: "至少填写一个明确目录。",
      },
    ],
    buildSpec: (values, platform) => {
      const paths = selectedList(values, "paths");
      if (paths.length === 0) {
        throw new UserFacingError("请至少指定一个允许目录");
      }
      return npxSpec("@modelcontextprotocol/server-filesystem", platform, {
        extraArgs: paths,
      });
    },
  }),
  catalogItem({
    id: "time",
    name: "Time",
    description: "查询时间与时区转换。",
    categories: ["basics"],
    tags: ["时间", "工具", "stdio"],
    publisher: "Model Context Protocol",
    providerGroup: "general",
    provenance: "reference",
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
    requirements: ["uv"],
    authLabel: "无",
    privilege: "read",
    fields: [],
    buildSpec: () => ({
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-time"],
    }),
  }),
  catalogItem({
    id: "memory",
    name: "Memory",
    description: "本地知识图谱记忆，供会话间回忆。",
    categories: ["basics"],
    tags: ["记忆", "stdio"],
    publisher: "Model Context Protocol",
    providerGroup: "general",
    provenance: "reference",
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
    requirements: ["node"],
    authLabel: "无",
    privilege: "write",
    fields: [],
    buildSpec: (_values, platform) =>
      npxSpec("@modelcontextprotocol/server-memory", platform),
  }),
  catalogItem({
    id: "fetch",
    name: "Fetch",
    description: "抓取网页内容供模型阅读。",
    categories: ["basics"],
    tags: ["网页", "抓取", "stdio"],
    publisher: "Model Context Protocol",
    providerGroup: "general",
    provenance: "reference",
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
    requirements: ["uv"],
    authLabel: "无",
    privilege: "read",
    risk: "可能访问本地或内部地址，请确认目标范围。",
    fields: [],
    buildSpec: () => ({
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-fetch"],
    }),
  }),
  catalogItem({
    id: "gitee",
    name: "Gitee MCP",
    description: "国内代码托管：仓库、Issue、Pull Request 与通知。",
    categories: ["china", "devtools"],
    tags: ["Gitee", "代码", "HTTP"],
    publisher: "Gitee",
    providerGroup: "china-other",
    provenance: "official",
    homepage: "https://help.gitee.com/ai-productivity/mcp-server",
    docs: "https://help.gitee.com/ai-productivity/mcp-server",
    requirements: ["none"],
    authLabel: "Access Token",
    privilege: "read",
    recommended: true,
    risk: "完整工具集含创建仓库、评论和合并 PR。默认只开放只读查询。",
    fields: [
      {
        key: "token",
        label: "个人访问令牌",
        type: "password",
        required: true,
        help: "在 Gitee 个人设置中创建。令牌写入请求头，不会进入搜索。",
      },
      {
        key: "access",
        label: "权限范围",
        type: "select",
        required: true,
        options: [
          { value: "readonly", label: "只读查询" },
          { value: "full", label: "完整工具集（含写入）" },
        ],
      },
    ],
    buildSpec: (values) => {
      const access = requiredText(values, "access", "权限范围");
      if (access !== "readonly" && access !== "full") {
        throw new UserFacingError("请选择有效的权限范围");
      }
      const headers: Record<string, string> = {
        Authorization: `Bearer ${requiredText(values, "token", "个人访问令牌")}`,
      };
      if (access === "readonly") {
        headers["X-MCP-Enabled-Tools"] = GITEE_READONLY_TOOLS;
      }
      return {
        type: "http",
        url: "https://api.gitee.com/mcp",
        headers,
      };
    },
  }),
  catalogItem({
    id: "tencent-docs",
    name: "腾讯文档 MCP",
    description: "在线文档与智能表格的查询、创建和编辑。",
    categories: ["china", "collab"],
    tags: ["腾讯文档", "办公", "HTTP"],
    publisher: "腾讯文档开放平台",
    providerGroup: "tencent",
    provenance: "official",
    homepage: "https://docs.qq.com/open/document/mcp/",
    docs: "https://docs.qq.com/open/document/mcp/",
    requirements: ["none"],
    authLabel: "MCP Token",
    privilege: "write",
    risk: "可读写腾讯文档空间数据；调用受会员档位与日限额约束。",
    fields: [
      {
        key: "token",
        label: "MCP Token",
        type: "password",
        required: true,
        help: "从腾讯文档「使用 MCP」页获取。请求头名称必须是 Authorization。",
      },
    ],
    buildSpec: (values) => ({
      type: "http",
      url: "https://docs.qq.com/openapi/mcp",
      headers: {
        Authorization: requiredText(values, "token", "MCP Token"),
      },
    }),
  }),
  catalogItem({
    id: "tapd",
    name: "TAPD MCP",
    description: "需求、缺陷、任务与迭代等国内研发协作。",
    categories: ["china", "devtools"],
    tags: ["TAPD", "研发", "stdio"],
    publisher: "腾讯 TAPD",
    providerGroup: "tencent",
    provenance: "official",
    homepage: "https://cloud.tencent.com/developer/mcp/server/11474",
    docs: "https://cloud.tencent.com/developer/mcp/server/11474",
    requirements: ["uv"],
    authLabel: "Access Token",
    privilege: "write",
    recommended: true,
    risk: "可读写需求、缺陷与任务。请使用个人访问令牌并限制项目范围。",
    fields: [
      {
        key: "token",
        label: "个人访问令牌",
        type: "password",
        required: true,
        help: "在 TAPD「我的设置 → 个人访问令牌」创建。",
      },
      {
        key: "workspaceId",
        label: "默认项目 ID",
        type: "text",
        help: "选填。填写后可少传 workspace_id。",
      },
    ],
    buildSpec: (values) => {
      const env: Record<string, string> = {
        TAPD_ACCESS_TOKEN: requiredText(values, "token", "个人访问令牌"),
      };
      const workspaceId = optionalText(values, "workspaceId");
      if (workspaceId) env.TAPD_DEFAULT_WORKSPACE_ID = workspaceId;
      return {
        type: "stdio",
        command: "uvx",
        args: ["mcp-server-tapd"],
        env,
      };
    },
  }),
  catalogItem({
    id: "caiyun-weather",
    name: "彩云天气 MCP",
    description: "中国天气、分钟级降水、预报与预警。",
    categories: ["china", "maps"],
    tags: ["天气", "生活", "HTTP"],
    publisher: "彩云科技",
    providerGroup: "china-other",
    provenance: "official",
    homepage: "https://docs.caiyunapp.com/weather-api/mcp.html",
    docs: "https://docs.caiyunapp.com/weather-api/mcp.html",
    requirements: ["none"],
    authLabel: "API Key",
    privilege: "read",
    recommended: true,
    fields: [
      {
        key: "apiKey",
        label: "彩云天气 API Key",
        type: "password",
        required: true,
        help: "在彩云开发者平台申请，写入 X-Caiyun-API-Key 请求头。",
      },
    ],
    buildSpec: (values) => ({
      type: "http",
      url: "https://mcp-weather.caiyunapp.com/mcp",
      headers: {
        "X-Caiyun-API-Key": requiredText(values, "apiKey", "彩云天气 API Key"),
      },
    }),
  }),
  catalogItem({
    id: "aliyun-websearch",
    name: "阿里云 WebSearch MCP",
    description: "国内联网检索，供编码 Agent 获取实时信息。",
    categories: ["china", "basics"],
    tags: ["搜索", "百炼", "HTTP"],
    publisher: "阿里云百炼",
    providerGroup: "alibaba",
    provenance: "official",
    homepage:
      "https://help.aliyun.com/zh/model-studio/web-search-for-coding-plan",
    docs: "https://help.aliyun.com/zh/model-studio/web-search-for-coding-plan",
    requirements: ["none"],
    authLabel: "百炼 API Key",
    privilege: "read",
    recommended: true,
    risk: "联网搜索可能产生服务调用费用。请使用百炼通用 API Key，不要使用 Token Plan 的 sk-sp- 前缀密钥。",
    fields: [
      {
        key: "apiKey",
        label: "百炼 API Key",
        type: "password",
        required: true,
        help: "使用百炼通用 sk- 密钥。",
      },
    ],
    buildSpec: (values) => ({
      type: "http",
      url: "https://dashscope.aliyuncs.com/api/v1/mcps/WebSearch/mcp",
      headers: {
        Authorization: `Bearer ${requiredText(values, "apiKey", "百炼 API Key")}`,
      },
    }),
  }),
  catalogItem({
    id: "yuque",
    name: "语雀 MCP",
    description: "知识库文档的查询、创建与更新。",
    categories: ["china", "collab"],
    tags: ["语雀", "文档", "stdio"],
    publisher: "语雀",
    providerGroup: "china-other",
    provenance: "official",
    homepage: "https://github.com/yuque/yuque-mcp-server",
    docs: "https://github.com/yuque/yuque-mcp-server/blob/main/README.zh-CN.md",
    requirements: ["node"],
    authLabel: "Access Token",
    privilege: "write",
    recommended: true,
    risk: "可读写语雀知识库。请使用个人访问令牌，不要把 Token 写进启动参数。",
    fields: [
      {
        key: "token",
        label: "个人访问令牌",
        type: "password",
        required: true,
        help: "在语雀「设置 → 开发者设置」创建。写入环境变量，不会进入搜索。",
      },
    ],
    buildSpec: (values, platform) =>
      npxSpec("yuque-mcp", platform, {
        env: {
          YUQUE_PERSONAL_TOKEN: requiredText(values, "token", "个人访问令牌"),
        },
      }),
  }),
  catalogItem({
    id: "apifox",
    name: "Apifox API 文档",
    description: "读取团队 API 文档、数据模型与测试用例。",
    categories: ["china", "devtools"],
    tags: ["Apifox", "API", "stdio"],
    publisher: "Apifox",
    providerGroup: "china-other",
    provenance: "official",
    homepage: "https://docs.apifox.com/apifox-mcp-server",
    docs: "https://docs.apifox.com/apifox-mcp-server",
    requirements: ["node"],
    authLabel: "Access Token",
    privilege: "write",
    risk: "可读取并操作指定项目的 API 文档。",
    fields: [
      {
        key: "token",
        label: "API 访问令牌",
        type: "password",
        required: true,
        help: "在 Apifox「账号设置 → API 访问令牌」创建。",
      },
      {
        key: "projectId",
        label: "项目 ID",
        type: "text",
        required: true,
        help: "只绑定一个项目，避免一次授予全部项目权限。",
      },
    ],
    buildSpec: (values, platform) =>
      npxSpec("apifox-mcp-server@latest", platform, {
        extraArgs: [
          `--project-id=${requiredText(values, "projectId", "项目 ID")}`,
        ],
        env: {
          APIFOX_ACCESS_TOKEN: requiredText(values, "token", "API 访问令牌"),
        },
      }),
  }),
  catalogItem({
    id: "antv-chart",
    name: "AntV 图表 MCP",
    description: "生成折线、柱状、饼图等图表图片。",
    categories: ["china", "basics"],
    tags: ["AntV", "图表", "stdio"],
    publisher: "AntV",
    providerGroup: "alibaba",
    provenance: "official",
    homepage: "https://github.com/antvis/mcp-server-chart",
    docs: "https://github.com/antvis/mcp-server-chart",
    requirements: ["node"],
    authLabel: "无",
    privilege: "read",
    recommended: true,
    fields: [],
    buildSpec: (_values, platform) =>
      npxSpec("@antv/mcp-server-chart", platform),
  }),
  catalogItem({
    id: "sequential-thinking",
    name: "Sequential Thinking",
    description: "把复杂问题拆成可修订的思考步骤。",
    categories: ["basics"],
    tags: ["推理", "工具", "stdio"],
    publisher: "Model Context Protocol",
    providerGroup: "general",
    provenance: "reference",
    homepage:
      "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
    requirements: ["node"],
    authLabel: "无",
    privilege: "read",
    fields: [],
    buildSpec: (_values, platform) =>
      npxSpec("@modelcontextprotocol/server-sequential-thinking", platform),
  }),
  catalogItem({
    id: "chrome-devtools",
    name: "Chrome DevTools MCP",
    description: "用本机 Chrome 做页面调试、性能分析和浏览器自动化。",
    categories: ["devtools"],
    tags: ["Chrome", "调试", "stdio"],
    publisher: "Chrome DevTools",
    providerGroup: "general",
    provenance: "official",
    homepage: "https://github.com/ChromeDevTools/chrome-devtools-mcp",
    docs: "https://developer.chrome.com/blog/chrome-devtools-mcp",
    requirements: ["node"],
    authLabel: "无",
    privilege: "write",
    risk: "可控制本机 Chrome，访问当前打开的页面与会话。",
    fields: [],
    buildSpec: (_values, platform) =>
      npxSpec("chrome-devtools-mcp@latest", platform),
  }),
  catalogItem({
    id: "git",
    name: "Git",
    description: "查看状态、差异、日志，并在本地仓库执行 Git 操作。",
    categories: ["devtools"],
    tags: ["Git", "版本控制", "stdio"],
    publisher: "Model Context Protocol",
    providerGroup: "general",
    provenance: "reference",
    homepage:
      "https://github.com/modelcontextprotocol/servers/tree/main/src/git",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/git",
    requirements: ["uv"],
    authLabel: "无",
    privilege: "write",
    risk: "可读取并修改本地 Git 仓库。",
    fields: [],
    buildSpec: () => ({
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-git"],
    }),
  }),
  catalogItem({
    id: "markitdown",
    name: "MarkItDown",
    description: "把 PDF、Office、图片等转成 Markdown。",
    categories: ["basics"],
    tags: ["文档", "Markdown", "stdio"],
    publisher: "Microsoft",
    providerGroup: "general",
    provenance: "official",
    homepage: "https://github.com/microsoft/markitdown",
    docs: "https://github.com/microsoft/markitdown/tree/main/packages/markitdown-mcp",
    requirements: ["uv"],
    authLabel: "无",
    privilege: "read",
    risk: "可读取本机或网络文件并转换。",
    fields: [],
    buildSpec: () => ({
      type: "stdio",
      command: "uvx",
      args: ["markitdown-mcp"],
    }),
  }),
  catalogItem({
    id: "edgeone-pages",
    name: "EdgeOne Pages MCP",
    description: "把 HTML 部署成可公开访问的预览链接，无需登录。",
    categories: ["china", "cloud"],
    tags: ["腾讯云", "部署", "HTTP"],
    publisher: "腾讯云 EdgeOne",
    providerGroup: "tencent",
    provenance: "official",
    homepage: "https://pages.edgeone.ai/document/pages-mcp",
    docs: "https://cloud.tencent.com.cn/developer/mcp/server/10011",
    requirements: ["none"],
    authLabel: "无",
    privilege: "write",
    risk: "会把 HTML 发布为公开访问链接。",
    fields: [],
    buildSpec: () => ({
      type: "http",
      url: "https://mcp-on-edge.edgeone.site/mcp-server",
    }),
  }),
  catalogItem({
    id: "howtocook",
    name: "HowToCook 菜谱 MCP",
    description: "中文菜谱推荐与膳食规划。",
    categories: ["china", "maps"],
    tags: ["菜谱", "生活", "stdio"],
    publisher: "HowToCook MCP",
    providerGroup: "china-other",
    provenance: "community",
    homepage: "https://github.com/worryzyy/howtocook-mcp",
    docs: "https://github.com/worryzyy/howtocook-mcp",
    requirements: ["node"],
    authLabel: "无",
    privilege: "read",
    fields: [],
    buildSpec: (_values, platform) => npxSpec("howtocook-mcp", platform),
  }),
  catalogItem({
    id: "train-12306",
    name: "12306 余票查询",
    description: "查询火车票余票、经停与中转。",
    categories: ["china", "maps"],
    tags: ["出行", "火车", "stdio"],
    publisher: "12306-mcp",
    providerGroup: "china-other",
    provenance: "community",
    homepage: "https://github.com/Joooook/12306-mcp",
    docs: "https://github.com/Joooook/12306-mcp",
    requirements: ["node"],
    authLabel: "无",
    privilege: "read",
    risk: "社区封装的公开查询，不是 12306 官方 MCP。",
    fields: [],
    buildSpec: (_values, platform) => npxSpec("12306-mcp", platform),
  }),
  catalogItem({
    id: "duckduckgo",
    name: "DuckDuckGo 搜索",
    description: "免费网页搜索，无需 API Key。",
    categories: ["basics"],
    tags: ["搜索", "stdio"],
    publisher: "duckduckgo-mcp-server",
    providerGroup: "general",
    provenance: "community",
    homepage: "https://github.com/nickclyde/duckduckgo-mcp-server",
    docs: "https://github.com/nickclyde/duckduckgo-mcp-server",
    requirements: ["uv"],
    authLabel: "无",
    privilege: "read",
    risk: "会向 DuckDuckGo 发起搜索请求。默认区域为中国。",
    fields: [],
    buildSpec: () => ({
      type: "stdio",
      command: "uvx",
      args: ["duckduckgo-mcp-server"],
      env: { DDG_REGION: "cn-zh" },
    }),
  }),
];

export const MCP_PROVENANCE_LABEL: Record<McpProvenance, string> = {
  official: "官方",
  reference: "官方参考实现",
  community: "社区",
};

export function findCatalogItem(id: string): McpCatalogItem | undefined {
  return MCP_CATALOG.find((item) => item.id === id);
}

export function catalogTransportLabel(item: McpCatalogItem): string {
  if (item.tags.includes("SSE")) return "SSE";
  if (item.tags.includes("HTTP")) return "HTTP";
  return "stdio";
}

export function catalogRequiresConfig(item: McpCatalogItem): boolean {
  return item.fields.length > 0;
}

export function catalogInstallModeLabel(item: McpCatalogItem): string {
  return catalogRequiresConfig(item) ? "配置安装" : "直接安装";
}

export function catalogSearchText(item: McpCatalogItem): string {
  return [
    item.id,
    item.name,
    item.description,
    item.publisher,
    item.authLabel,
    item.disabledReason ?? "",
    catalogInstallModeLabel(item),
    MCP_CATALOG_PROVIDERS.find((entry) => entry.id === item.providerGroup)
      ?.label ?? "",
    ...item.tags,
    ...item.categories.map((category) => MCP_CATEGORY_LABEL[category]),
  ]
    .join("\n")
    .toLocaleLowerCase();
}

export function catalogRecipeIdentity(
  item: McpCatalogItem,
  platform: McpLaunchPlatform,
): string {
  if (!item.installable) {
    return `listed:${item.id}`;
  }
  const placeholders: McpInstallValues = {};
  for (const field of item.fields) {
    if (field.type === "multi-select") {
      placeholders[field.key] = field.options?.[0]
        ? [field.options[0].value]
        : ["placeholder"];
      continue;
    }
    if (field.type === "path") {
      placeholders[field.key] = ["C:\\catalog-placeholder"];
      continue;
    }
    if (field.type === "select") {
      placeholders[field.key] = field.options?.[0]?.value ?? "placeholder";
      continue;
    }
    placeholders[field.key] = "placeholder";
  }
  return mcpRecipeIdentity(
    item.build(placeholders, ["claude"], platform).server,
  );
}

export function matchesCatalogRecipe(
  item: McpCatalogItem,
  server: McpServer,
  platform: McpLaunchPlatform,
): boolean {
  if (!item.installable) return false;
  return (
    mcpRecipeIdentity(server.server) === catalogRecipeIdentity(item, platform)
  );
}
