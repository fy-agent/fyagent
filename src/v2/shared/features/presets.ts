import { buildNpxCommand } from "./mcpLaunch";
import type { McpServer } from "./types";

export interface McpPreset {
  id: string;
  name: string;
  server: McpServer["server"];
  tags?: string[];
  homepage?: string;
  docs?: string;
  source?: string;
}

function npxPreset(
  id: string,
  packageName: string,
  tags: string[],
  docs: string,
  homepage: string = docs,
): McpPreset {
  const command = buildNpxCommand(packageName);
  return {
    id,
    name: packageName,
    tags,
    server: { type: "stdio", ...command },
    homepage,
    docs,
  };
}

export const mcpPresets: readonly McpPreset[] = [
  {
    id: "fetch",
    name: "mcp-server-fetch",
    tags: ["stdio", "http", "web"],
    server: { type: "stdio", command: "uvx", args: ["mcp-server-fetch"] },
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
  },
  {
    id: "time",
    name: "mcp-server-time",
    tags: ["stdio", "time", "utility"],
    server: { type: "stdio", command: "uvx", args: ["mcp-server-time"] },
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
  },
  npxPreset(
    "memory",
    "@modelcontextprotocol/server-memory",
    ["stdio", "memory", "graph"],
    "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
    "https://github.com/modelcontextprotocol/servers",
  ),
  npxPreset(
    "sequential-thinking",
    "@modelcontextprotocol/server-sequential-thinking",
    ["stdio", "thinking", "reasoning"],
    "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
    "https://github.com/modelcontextprotocol/servers",
  ),
  npxPreset(
    "context7",
    "@upstash/context7-mcp",
    ["stdio", "docs", "search"],
    "https://github.com/upstash/context7/blob/master/README.md",
    "https://context7.com",
  ),
];
