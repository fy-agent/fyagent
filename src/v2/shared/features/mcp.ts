import type { McpAssignments } from "./assignments";

export interface McpServerSpec extends Record<string, unknown> {
  type?: "stdio" | "http" | "sse";
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  url?: string;
  headers?: Record<string, string>;
}

export interface McpServer extends Record<string, unknown> {
  id: string;
  name: string;
  server: McpServerSpec;
  apps: McpAssignments;
  description?: string;
  tags?: string[];
  homepage?: string;
  docs?: string;
  source?: string;
}

export type McpServersMap = Record<string, McpServer>;
