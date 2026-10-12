import { describe, expect, it } from "vitest";
import {
  buildMcpInspectorConfig,
  MCP_INSPECTOR_INITIALIZE_COMMAND,
  MCP_INSPECTOR_TOOLS_COMMAND,
} from "@/pages/mcp/inspector";

describe("local Inspector configuration", () => {
  it("exports only stdio launch fields and preserves credentials and literal arguments", () => {
    const spec = {
      command: "node",
      args: ["server.js", "$(literal)"],
      env: { API_KEY: "fixture-secret" },
      cwd: "C:\\MCP",
      extra: "unused",
    };
    expect(JSON.parse(buildMcpInspectorConfig(spec))).toEqual({
      mcpServers: {
        "fyagent-diagnostic": {
          type: "stdio",
          command: "node",
          args: spec.args,
          env: spec.env,
          cwd: spec.cwd,
          connectionTimeout: 15000,
          requestTimeout: 15000,
        },
      },
    });
    expect(spec.extra).toBe("unused");
  });
  it.each(["http", "sse"] as const)(
    "exports %s endpoints and authorization headers without stdio fields",
    (type) => {
      expect(
        JSON.parse(
          buildMcpInspectorConfig({
            type,
            url: "http://localhost:3001/mcp",
            headers: { Authorization: "Bearer fixture-secret" },
            command: "unused",
          }),
        ),
      ).toEqual({
        mcpServers: {
          "fyagent-diagnostic": {
            type,
            url: "http://localhost:3001/mcp",
            headers: { Authorization: "Bearer fixture-secret" },
            connectionTimeout: 15000,
            requestTimeout: 15000,
          },
        },
      });
    },
  );
  it("rejects missing launch information and exposes only initialize and tools/list commands", () => {
    expect(() => buildMcpInspectorConfig({})).toThrow(
      "请先补全 MCP 命令或地址",
    );
    expect(() => buildMcpInspectorConfig({ type: "http" })).toThrow(
      "请先补全 MCP 命令或地址",
    );
    expect(MCP_INSPECTOR_INITIALIZE_COMMAND).toContain("--method initialize");
    expect(MCP_INSPECTOR_TOOLS_COMMAND).toContain("--method tools/list");
    expect(MCP_INSPECTOR_TOOLS_COMMAND).not.toContain("tools/call");
  });
});
