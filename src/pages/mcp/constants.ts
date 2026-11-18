import type { McpServer, McpTargetId } from "../../shared/features/types";

export const DEFAULT_NEW_APPS: readonly McpTargetId[] = [];

export function transportOf(server: McpServer): "stdio" | "http" | "sse" {
  if (server.server.type === "http" || server.server.type === "sse") {
    return server.server.type;
  }
  return "stdio";
}
