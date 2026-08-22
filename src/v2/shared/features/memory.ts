export const MEMORY_DOCUMENT_IDS = [
  "openclaw-memory",
  "openclaw-user",
  "hermes-memory",
  "hermes-user",
] as const;

export type MemoryDocumentId = (typeof MEMORY_DOCUMENT_IDS)[number];
export const HERMES_MEMORY_KINDS = ["memory", "user"] as const;
export type HermesMemoryKind = (typeof HERMES_MEMORY_KINDS)[number];

export interface HermesMemoryLimits {
  memory: number;
  user: number;
  memoryEnabled: boolean;
  userEnabled: boolean;
}

export interface DailyMemoryFileInfo {
  filename: string;
  date: string;
  sizeBytes: number;
  modifiedAt: number;
  preview: string;
}

export interface DailyMemorySearchResult {
  filename: string;
  date: string;
  sizeBytes: number;
  modifiedAt: number;
  snippet: string;
  matchCount: number;
}

export type OpenClawDirectory = "workspace" | "memory";
