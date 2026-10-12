import type { McpServerSpec } from "../../shared/features/mcp";

export const MCP_INSPECTOR_INITIALIZE_COMMAND =
  "npx @modelcontextprotocol/inspector --cli --config mcp-inspector.json --server fyagent-diagnostic --method initialize --connect-timeout 15000";
export const MCP_INSPECTOR_TOOLS_COMMAND =
  "npx @modelcontextprotocol/inspector --cli --config mcp-inspector.json --server fyagent-diagnostic --method tools/list --connect-timeout 15000";

export function buildMcpInspectorConfig(spec: McpServerSpec): string {
  const type = spec.type ?? "stdio";
  if (
    !["stdio", "http", "sse"].includes(type) ||
    (type === "stdio" ? !spec.command?.trim() : !spec.url?.trim())
  ) {
    throw new Error("请先补全 MCP 命令或地址");
  }
  const server =
    type === "stdio"
      ? {
          type,
          command: spec.command,
          args: spec.args,
          env: spec.env,
          cwd: spec.cwd,
        }
      : { type, url: spec.url, headers: spec.headers };
  return JSON.stringify(
    {
      mcpServers: {
        "fyagent-diagnostic": {
          ...server,
          connectionTimeout: 15000,
          requestTimeout: 15000,
        },
      },
    },
    null,
    2,
  );
}
