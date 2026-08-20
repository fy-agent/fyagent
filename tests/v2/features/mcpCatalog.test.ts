import { describe, expect, it } from "vitest";

import {
  MCP_CATALOG,
  catalogRequiresConfig,
  catalogSearchText,
  findCatalogItem,
} from "@/v2/pages/mcp/catalog";
import { DEFAULT_NEW_APPS } from "@/v2/pages/mcp/constants";
import { mcpPresets } from "@/v2/shared/features/presets";
import { UserFacingError } from "@/v2/shared/features/helpers";
import {
  mcpUrlSearchToken,
  redactMcpArgs,
} from "@/v2/shared/features/mcpSecurity";

function item(id: string) {
  const catalogItem = MCP_CATALOG.find((entry) => entry.id === id);
  if (!catalogItem) throw new Error(`missing catalog item ${id}`);
  return catalogItem;
}

describe("MCP curated catalog", () => {
  it("ships only installable curated items", () => {
    expect(MCP_CATALOG.map((entry) => entry.id)).toEqual([
      "amap",
      "baidu-map",
      "feishu",
      "dingtalk",
      "yunxiao",
      "context7",
      "playwright",
      "filesystem",
      "time",
      "memory",
      "fetch",
      "gitee",
      "tencent-docs",
      "tapd",
      "caiyun-weather",
      "aliyun-websearch",
      "yuque",
      "apifox",
      "antv-chart",
      "sequential-thinking",
      "chrome-devtools",
      "git",
      "markitdown",
      "edgeone-pages",
      "howtocook",
      "train-12306",
      "duckduckgo",
    ]);
    expect(MCP_CATALOG).toHaveLength(27);
    expect(MCP_CATALOG.every((entry) => entry.installable)).toBe(true);
    expect(catalogRequiresConfig(item("playwright"))).toBe(false);
    expect(catalogRequiresConfig(item("antv-chart"))).toBe(false);
    expect(catalogRequiresConfig(item("amap"))).toBe(true);
    expect(catalogRequiresConfig(item("gitee"))).toBe(true);
    expect(findCatalogItem("time")?.name).toBe("Time");
    expect(findCatalogItem("unknown-server")).toBeUndefined();
    expect(findCatalogItem("tencent-maps")).toBeUndefined();
    expect(findCatalogItem("minimax")).toBeUndefined();
  });

  it("defaults new MCP installs to Agent catalog target order", () => {
    expect(DEFAULT_NEW_APPS).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude",
      "opencode",
    ]);
  });

  it("builds Windows and macOS npx commands", () => {
    const playwright = item("playwright");
    expect(playwright.build({}, DEFAULT_NEW_APPS, "windows").server).toEqual({
      type: "stdio",
      command: "cmd",
      args: ["/c", "npx", "-y", "@playwright/mcp@latest"],
    });
    expect(playwright.build({}, DEFAULT_NEW_APPS, "macos").server).toEqual({
      type: "stdio",
      command: "npx",
      args: ["-y", "@playwright/mcp@latest"],
    });
    expect(playwright.build({}, DEFAULT_NEW_APPS, "unknown").server).toEqual({
      type: "stdio",
      command: "npx",
      args: ["-y", "@playwright/mcp@latest"],
    });
  });

  it("requires business fields before building credentialed items", () => {
    expect(() => item("amap").build({}, DEFAULT_NEW_APPS, "macos")).toThrow(
      UserFacingError,
    );
    expect(() =>
      item("filesystem").build({ paths: [] }, DEFAULT_NEW_APPS, "macos"),
    ).toThrow(UserFacingError);
    expect(() =>
      item("dingtalk").build(
        {
          clientId: "id",
          clientSecret: "secret",
          profiles: ["ALL"],
        },
        DEFAULT_NEW_APPS,
        "macos",
      ),
    ).toThrow(UserFacingError);
  });

  it("puts the Amap key in the URL and keeps it out of the search token", () => {
    const server = item("amap").build(
      { key: "amap-query-secret" },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(server.server).toEqual({
      type: "http",
      url: "https://mcp.amap.com/mcp?key=amap-query-secret",
    });
    expect(mcpUrlSearchToken(server.server.url ?? "")).toBe(
      "https://mcp.amap.com/mcp",
    );
  });

  it("masks the Feishu app secret in display args", () => {
    const server = item("feishu").build(
      { appId: "cli_app", appSecret: "feishu-app-secret" },
      DEFAULT_NEW_APPS,
      "windows",
    );
    expect(server.server.command).toBe("cmd");
    expect(redactMcpArgs(server.server.args ?? [])).toEqual([
      "/c",
      "npx",
      "-y",
      "@larksuiteoapi/lark-mcp",
      "mcp",
      "-a",
      "cli_app",
      "-s",
      "••••••",
    ]);
    expect(JSON.stringify(server.apps)).toEqual(
      JSON.stringify({
        qoderwork: true,
        "trae-work": true,
        workbuddy: true,
        grokbuild: true,
        codex: true,
        claude: true,
        opencode: true,
      }),
    );
  });

  it("stores Baidu and DingTalk secrets in env rather than the command line", () => {
    const baidu = item("baidu-map").build(
      { apiKey: "baidu-ak" },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(baidu.server.env).toEqual({ BAIDU_MAP_API_KEY: "baidu-ak" });
    const dingtalk = item("dingtalk").build(
      {
        clientId: "ding-id",
        clientSecret: "ding-secret",
        profiles: ["chatbot", "calendar"],
      },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(dingtalk.server.env).toEqual({
      DINGTALK_Client_ID: "ding-id",
      DINGTALK_Client_Secret: "ding-secret",
      ACTIVE_PROFILES: "chatbot,calendar",
    });
    expect(dingtalk.server.env?.ACTIVE_PROFILES).not.toBe("ALL");
  });

  it("builds Yunxiao and Context7 as Streamable HTTP", () => {
    const yunxiao = item("yunxiao").build(
      { token: "yunxiao-token", toolsets: ["codeup", "flow"] },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(yunxiao.server).toEqual({
      type: "http",
      url: "https://openapi-rdc.aliyuncs.com/ai/mcp?toolsets=codeup%2Cflow",
      headers: { Authorization: "Bearer yunxiao-token" },
    });
    const context7 = item("context7").build({}, DEFAULT_NEW_APPS, "macos");
    expect(context7.server).toEqual({
      type: "http",
      url: "https://mcp.context7.com/mcp",
    });
  });

  it("keeps time and fetch catalog commands aligned with presets", () => {
    const time = item("time").build({}, DEFAULT_NEW_APPS, "windows");
    const fetch = item("fetch").build({}, DEFAULT_NEW_APPS, "windows");
    expect(mcpPresets.find((preset) => preset.id === "time")?.server).toEqual(
      time.server,
    );
    expect(mcpPresets.find((preset) => preset.id === "fetch")?.server).toEqual(
      fetch.server,
    );
    expect(
      item("memory").build({}, DEFAULT_NEW_APPS, "windows").server,
    ).toEqual({
      type: "stdio",
      command: "cmd",
      args: ["/c", "npx", "-y", "@modelcontextprotocol/server-memory"],
    });
  });

  it("builds China P0 HTTP recipes without putting secrets in search text", () => {
    const gitee = item("gitee").build(
      { token: "gitee-pat", access: "readonly" },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(gitee.server).toEqual({
      type: "http",
      url: "https://api.gitee.com/mcp",
      headers: {
        Authorization: "Bearer gitee-pat",
        "X-MCP-Enabled-Tools": expect.stringContaining("list_user_repos"),
      },
    });
    expect(catalogSearchText(item("gitee"))).not.toContain("gitee-pat");
    expect(
      item("gitee").build(
        { token: "gitee-pat", access: "full" },
        DEFAULT_NEW_APPS,
        "macos",
      ).server.headers,
    ).toEqual({ Authorization: "Bearer gitee-pat" });

    const docs = item("tencent-docs").build(
      { token: "docs-mcp-token" },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(docs.server).toEqual({
      type: "http",
      url: "https://docs.qq.com/openapi/mcp",
      headers: { Authorization: "docs-mcp-token" },
    });

    const weather = item("caiyun-weather").build(
      { apiKey: "caiyun-key" },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(weather.server).toEqual({
      type: "http",
      url: "https://mcp-weather.caiyunapp.com/mcp",
      headers: { "X-Caiyun-API-Key": "caiyun-key" },
    });

    const search = item("aliyun-websearch").build(
      { apiKey: "sk-websearch" },
      DEFAULT_NEW_APPS,
      "macos",
    );
    expect(search.server).toEqual({
      type: "http",
      url: "https://dashscope.aliyuncs.com/api/v1/mcps/WebSearch/mcp",
      headers: { Authorization: "Bearer sk-websearch" },
    });
    expect(mcpUrlSearchToken(search.server.url ?? "")).toBe(
      "https://dashscope.aliyuncs.com/api/v1/mcps/WebSearch/mcp",
    );
  });

  it("builds TAPD, Yuque, Apifox, and zero-config stdio recipes", () => {
    expect(
      item("tapd").build(
        { token: "tapd-token", workspaceId: "101" },
        DEFAULT_NEW_APPS,
        "windows",
      ).server,
    ).toEqual({
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-tapd"],
      env: {
        TAPD_ACCESS_TOKEN: "tapd-token",
        TAPD_DEFAULT_WORKSPACE_ID: "101",
      },
    });
    expect(
      item("yuque").build({ token: "yuque-token" }, DEFAULT_NEW_APPS, "macos")
        .server,
    ).toEqual({
      type: "stdio",
      command: "npx",
      args: ["-y", "yuque-mcp"],
      env: { YUQUE_PERSONAL_TOKEN: "yuque-token" },
    });
    expect(
      item("apifox").build(
        { token: "apifox-token", projectId: "123456" },
        DEFAULT_NEW_APPS,
        "windows",
      ).server,
    ).toEqual({
      type: "stdio",
      command: "cmd",
      args: [
        "/c",
        "npx",
        "-y",
        "apifox-mcp-server@latest",
        "--project-id=123456",
      ],
      env: { APIFOX_ACCESS_TOKEN: "apifox-token" },
    });
    expect(
      item("antv-chart").build({}, DEFAULT_NEW_APPS, "windows").server,
    ).toEqual({
      type: "stdio",
      command: "cmd",
      args: ["/c", "npx", "-y", "@antv/mcp-server-chart"],
    });
    expect(
      item("sequential-thinking").build({}, DEFAULT_NEW_APPS, "macos").server,
    ).toEqual({
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-sequential-thinking"],
    });
    expect(
      item("chrome-devtools").build({}, DEFAULT_NEW_APPS, "windows").server,
    ).toEqual({
      type: "stdio",
      command: "cmd",
      args: ["/c", "npx", "-y", "chrome-devtools-mcp@latest"],
    });
    expect(item("git").build({}, DEFAULT_NEW_APPS, "macos").server).toEqual({
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-git"],
    });
    expect(
      item("markitdown").build({}, DEFAULT_NEW_APPS, "windows").server,
    ).toEqual({
      type: "stdio",
      command: "uvx",
      args: ["markitdown-mcp"],
    });
    expect(
      item("edgeone-pages").build({}, DEFAULT_NEW_APPS, "macos").server,
    ).toEqual({
      type: "http",
      url: "https://mcp-on-edge.edgeone.site/mcp-server",
    });
    expect(
      item("howtocook").build({}, DEFAULT_NEW_APPS, "macos").server,
    ).toEqual({
      type: "stdio",
      command: "npx",
      args: ["-y", "howtocook-mcp"],
    });
    expect(
      item("train-12306").build({}, DEFAULT_NEW_APPS, "windows").server,
    ).toEqual({
      type: "stdio",
      command: "cmd",
      args: ["/c", "npx", "-y", "12306-mcp"],
    });
    expect(
      item("duckduckgo").build({}, DEFAULT_NEW_APPS, "macos").server,
    ).toEqual({
      type: "stdio",
      command: "uvx",
      args: ["duckduckgo-mcp-server"],
      env: { DDG_REGION: "cn-zh" },
    });
  });
});
