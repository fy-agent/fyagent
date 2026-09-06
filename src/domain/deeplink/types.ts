/** Native deep-link payload shape for pure preview/security validation.
 * Receiving or decoding this data never authorizes a native write. */
export type ResourceType = "provider" | "prompt" | "mcp" | "skill";

export interface DeepLinkImportRequest {
  version: string;
  resource: ResourceType;
  app?:
    | "claude"
    | "codex"
    | "gemini"
    | "grokbuild"
    | "opencode"
    | "openclaw"
    | "hermes";
  name?: string;
  enabled?: boolean;
  // Only an in-app user confirmation may approve activation; a URL cannot.
  activationApproved?: boolean;
  homepage?: string;
  endpoint?: string;
  apiKey?: string;
  icon?: string;
  model?: string;
  notes?: string;
  haikuModel?: string;
  sonnetModel?: string;
  opusModel?: string;
  content?: string;
  description?: string;
  apps?: string;
  repo?: string;
  directory?: string;
  branch?: string;
  config?: string;
  configFormat?: string;
  configUrl?: string;
  usageEnabled?: boolean;
  usageScript?: string;
  usageApiKey?: string;
  usageBaseUrl?: string;
  usageAccessToken?: string;
  usageUserId?: string;
  usageAutoInterval?: number;
}

export interface McpImportResult {
  importedCount: number;
  importedIds: string[];
  failed: Array<{ id: string; error: string }>;
}

export type ImportResult =
  | { type: "provider"; id: string }
  | { type: "prompt"; id: string }
  | ({ type: "mcp" } & McpImportResult)
  | { type: "skill"; key: string };
