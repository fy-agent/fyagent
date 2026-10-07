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
  /** Native import identities, independent of assignment flags and catalogue metadata. */
  sources?: McpImportSourceId[];
}

export type McpServersMap = Record<string, McpServer>;

export const MCP_IMPORT_SOURCES = [
  { id: "claude", label: "Claude Code" },
  { id: "codex", label: "Codex" },
  { id: "gemini", label: "Gemini" },
  { id: "grokbuild", label: "Grok Build" },
  { id: "opencode", label: "OpenCode" },
  { id: "hermes", label: "Hermes" },
  { id: "workbuddy", label: "WorkBuddy" },
  { id: "qoderwork", label: "QoderWork" },
  { id: "trae-work", label: "TRAE Work" },
] as const;

export type McpImportSourceId = (typeof MCP_IMPORT_SOURCES)[number]["id"];

export interface McpImportSourceResult {
  source: McpImportSourceId;
  added: number;
  assignmentChanged: number;
  unchanged: number;
  disabledSkipped: number;
  failureCode: "source_failed" | null;
}

export interface McpImportReport {
  contractVersion: 1;
  sources: McpImportSourceResult[];
}
