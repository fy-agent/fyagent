import { describe, expect, it, vi } from "vitest";

import {
  buildMcpSearchText,
  convergeSelection,
  isDiscoverableInstalled,
  mcpInstallDirectory,
  overlayKnownMcpFields,
  parseAdvancedServerJson,
  parseKeyValueLines,
  runSequentialBulk,
  sanitizeMcpConfigurationError,
  skillInstallPath,
} from "@/v2/shared/features/helpers";
import {
  createAssignments,
  type DiscoverableSkill,
  type InstalledSkill,
  type McpServer,
} from "@/v2/shared/features/types";

describe("V2 feature helpers", () => {
  it("converges selection to a visible item", () => {
    const items = [{ id: "a" }, { id: "b" }];
    expect(convergeSelection(items, "b")).toBe("b");
    expect(convergeSelection(items, "gone")).toBe("a");
    expect(convergeSelection([], "gone")).toBeNull();
  });

  it("never adds MCP env or headers to searchable text", () => {
    const server: McpServer = {
      id: "demo",
      name: "Visible",
      description: "Safe",
      apps: createAssignments(),
      server: {
        type: "stdio",
        command: "npx",
        env: { SECRET_TOKEN: "ultra-secret-value" },
        headers: { Authorization: "Bearer private-token" },
      },
    };
    const text = buildMcpSearchText(server);
    expect(text).toContain("visible");
    expect(text).toContain("npx");
    expect(text).not.toContain("secret");
    expect(text).not.toContain("private-token");
    expect(text).not.toContain("authorization");
  });

  it("does not index MCP URL query secrets or sensitive arguments", () => {
    const server: McpServer = {
      id: "maps",
      name: "Maps",
      apps: createAssignments(),
      server: {
        type: "http",
        url: "https://mcp.amap.com/mcp?key=amap-query-secret",
        args: ["mcp", "-a", "cli_app", "-s", "feishu-app-secret"],
      },
    };
    const text = buildMcpSearchText(server);
    expect(text).toContain("https://mcp.amap.com/mcp");
    expect(text).toContain("-a");
    expect(text).not.toContain("amap-query-secret");
    expect(text).not.toContain("feishu-app-secret");
    expect(text).not.toContain("key=amap");
  });

  it("does not echo secret-bearing MCP configuration errors", () => {
    expect(
      sanitizeMcpConfigurationError(
        new Error("Authorization header contains secret-token"),
      ),
    ).toBe("MCP 配置中的敏感字段未通过校验，请检查对应字段格式");
    expect(sanitizeMcpConfigurationError(new Error("URL is required"))).toBe(
      "MCP 配置中的 URL 未通过校验，请检查连接地址",
    );
    expect(
      sanitizeMcpConfigurationError(
        new Error("value xyz-unknown was rejected"),
      ),
    ).toBe("MCP 配置保存失败，请检查服务器字段");
  });

  it("parses env and headers at the required earliest separator", () => {
    expect(parseKeyValueLines("TOKEN=a=b", "env")).toEqual({
      value: { TOKEN: "a=b" },
      errors: [],
    });
    expect(parseKeyValueLines("Authorization: Bearer=a", "headers")).toEqual({
      value: { Authorization: "Bearer=a" },
      errors: [],
    });
    expect(parseKeyValueLines("malformed", "env").errors).toEqual([
      "第 1 行格式无效",
    ]);
  });

  it("rejects containers and preserves extensions during quick overlays", () => {
    expect(() => parseAdvancedServerJson('{"mcpServers":{}}')).toThrow(
      "完整配置列表",
    );
    expect(
      overlayKnownMcpFields(
        { command: "old", env: { SECRET: "x" }, extension: { keep: true } },
        { type: "http", url: "https://example.com" },
      ),
    ).toEqual({
      extension: { keep: true },
      type: "http",
      url: "https://example.com",
    });
  });

  it("runs bulk operations sequentially and reports partial failure", async () => {
    const order: string[] = [];
    const operation = vi.fn(async (id: string) => {
      order.push(id);
      if (id === "b") throw new Error("failed");
    });
    const result = await runSequentialBulk(["a", "b", "c"], operation);
    expect(order).toEqual(["a", "b", "c"]);
    expect(result.successes).toEqual(["a", "c"]);
    expect(result.failures).toEqual([{ id: "b", error: "请稍后重试。" }]);
  });

  it("prefers the resolved Skill install path over the directory name", () => {
    expect(
      skillInstallPath({
        directory: "review-skill",
        path: "C:/Users/xk/.fyagent/skills/review-skill",
      }),
    ).toBe("C:/Users/xk/.fyagent/skills/review-skill");
    expect(skillInstallPath({ directory: "review-skill" })).toBe(
      "review-skill",
    );
    expect(skillInstallPath({ directory: "review-skill", path: "  " })).toBe(
      "review-skill",
    );
  });

  it("derives an MCP install directory from cwd or an absolute command", () => {
    expect(mcpInstallDirectory({ command: "npx" })).toBeNull();
    expect(mcpInstallDirectory({ command: "uvx" })).toBeNull();
    expect(mcpInstallDirectory({ command: "cmd" })).toBeNull();
    expect(
      mcpInstallDirectory({
        command:
          "C:\\Users\\xk\\AppData\\Local\\OpenAI\\Codex\\runtimes\\cua_node\\node.exe",
      }),
    ).toBe("C:\\Users\\xk\\AppData\\Local\\OpenAI\\Codex\\runtimes\\cua_node");
    expect(mcpInstallDirectory({ command: "C:\\node.exe" })).toBe("C:\\");
    expect(mcpInstallDirectory({ command: "/usr/local/bin/node" })).toBe(
      "/usr/local/bin",
    );
    expect(
      mcpInstallDirectory({
        command: "npx",
        cwd: "D:\\workspace\\mcp-tools",
      }),
    ).toBe("D:\\workspace\\mcp-tools");
  });

  it("matches discoverable installs by directory tail and owner/name", () => {
    const discoverable: DiscoverableSkill = {
      key: "acme/skills:review-skill",
      name: "Review",
      description: "",
      directory: "skills/Review-Skill",
      repoOwner: "Acme",
      repoName: "Skills",
      repoBranch: "main",
    };
    const installed: InstalledSkill[] = [
      {
        id: "acme/skills:review-skill",
        name: "Review",
        directory: "Review-Skill",
        repoOwner: "acme",
        repoName: "skills",
        apps: createAssignments(["claude"]),
        installedAt: 1,
        updatedAt: 1,
      },
    ];
    expect(isDiscoverableInstalled(discoverable, installed)).toBe(true);
    expect(
      isDiscoverableInstalled(discoverable, [
        { ...installed[0], repoOwner: "other" },
      ]),
    ).toBe(false);
    expect(
      isDiscoverableInstalled(discoverable, [
        { ...installed[0], repoOwner: undefined, repoName: undefined },
      ]),
    ).toBe(false);
  });

  it("matches Skill 市场 installs by market owner and slug", () => {
    const discoverable: DiscoverableSkill = {
      key: "skillhub:tencent-docs",
      name: "腾讯文档",
      description: "中文介绍",
      directory: "tencent-docs",
      repoOwner: "skillhub.cn",
      repoName: "tencent-docs",
      repoBranch: "skillhub",
    };
    const installed: InstalledSkill[] = [
      {
        id: "skillhub:tencent-docs",
        name: "腾讯文档 TENCENT DOCS",
        directory: "腾讯文档-TENCENT-DOCS",
        repoOwner: "skillhub.cn",
        repoName: "tencent-docs",
        apps: createAssignments(["claude"]),
        installedAt: 1,
        updatedAt: 1,
      },
    ];
    expect(isDiscoverableInstalled(discoverable, installed)).toBe(true);
    expect(
      isDiscoverableInstalled(discoverable, [
        { ...installed[0], id: "skillhub:other", repoName: "other" },
      ]),
    ).toBe(false);
  });
});
