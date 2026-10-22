export const OMP_PROVIDER_APIS = [
  "openai-completions",
  "openai-responses",
] as const;
export type OmpProviderApi = (typeof OMP_PROVIDER_APIS)[number];

export const OMP_AUTH_MODES = ["apiKey", "none", "oauth"] as const;
export type OmpAuthMode = (typeof OMP_AUTH_MODES)[number];

export const OMP_FILE_STATUSES = [
  "ok",
  "notFound",
  "legacyJsonOnly",
  "invalidYaml",
] as const;
export type OmpFileStatus = (typeof OMP_FILE_STATUSES)[number];

export const OMP_CHANGED_FILES = ["models", "config"] as const;
export type OmpChangedFile = (typeof OMP_CHANGED_FILES)[number];

export interface OmpModelSummary {
  id: string;
  name: string | null;
}

/** Redacted provider view. Key material and command text never cross IPC. */
export interface OmpProviderSummary {
  id: string;
  baseUrl: string | null;
  api: string | null;
  auth: string | null;
  hasApiKey: boolean;
  hasCommandKey: boolean;
  models: OmpModelSummary[];
}

export interface OmpConfigRead {
  profile: string;
  modelsPath: string;
  configPath: string;
  modelsRevision: string;
  configRevision: string;
  providers: OmpProviderSummary[];
  defaultModel: string | null;
  modelsFileStatus: OmpFileStatus;
  configFileStatus: OmpFileStatus;
}

export interface OmpModelInput {
  id: string;
  name?: string | null;
}

export interface OmpProviderInput {
  id: string;
  baseUrl: string;
  api: OmpProviderApi;
  /**
   * Environment-variable name or `!command` reference. Absent/null keeps the
   * file value. Private form intent: never cached, previewed, or logged.
   */
  apiKey?: string | null;
  /** Absent/null keeps the file value; new providers default to apiKey. */
  auth?: OmpAuthMode | null;
  /** Merged by model ID; models not listed here are preserved. */
  models: OmpModelInput[];
}

export interface OmpSaveRequest {
  provider: OmpProviderInput;
  /** Absent/null keeps config; `provider/model` updates the default role. */
  defaultModel?: string | null;
  expectedModelsRevision: string;
  expectedConfigRevision: string;
}

export interface OmpSavePreview {
  planId: string;
  planDigest: string;
  profile: string;
  modelsPath: string;
  configPath: string;
  provider: OmpProviderSummary;
  defaultModel: string | null;
  changedFiles: OmpChangedFile[];
  warnings: string[];
}

export interface OmpApplyConfirmation {
  planId: string;
  planDigest: string;
}

export interface OmpSaveResult {
  modelsPath: string;
  configPath: string;
  modelsRevision: string;
  configRevision: string;
  provider: OmpProviderSummary;
  defaultModel: string | null;
  modelsWritten: boolean;
  configWritten: boolean;
}

export interface OmpCliDiscovery {
  detected: boolean;
  version: string | null;
  /** Read-only display metadata; never executed by FyAgent. */
  executablePath: string | null;
}

export interface OmpModelsPort {
  readConfig(): Promise<OmpConfigRead>;
  discoverCli(): Promise<OmpCliDiscovery>;
  previewSave(request: OmpSaveRequest): Promise<OmpSavePreview>;
  applySave(confirmation: OmpApplyConfirmation): Promise<OmpSaveResult>;
}

export const OMP_ERROR_CODES = [
  "stale",
  "invalid_input",
  "config",
  "unknown",
] as const;
export type OmpErrorCode = (typeof OMP_ERROR_CODES)[number];

export class OmpPortError extends Error {
  constructor(readonly code: OmpErrorCode) {
    super(`omp:${code}`);
    this.name = "OmpPortError";
  }
}

export function ompErrorCode(cause: unknown): OmpErrorCode {
  return cause instanceof OmpPortError ? cause.code : "unknown";
}

/** Maps the native `AppError` variant prefix to a closed code; raw text is dropped. */
export function classifyOmpNativeError(cause: unknown): OmpErrorCode {
  const text =
    typeof cause === "string"
      ? cause
      : cause instanceof Error
        ? cause.message
        : "";
  if (text.startsWith("并发冲突:")) return "stale";
  if (text.startsWith("无效输入:")) return "invalid_input";
  if (text.startsWith("配置错误:")) return "config";
  return "unknown";
}

/** Default suggestion for new provider IDs; not a validation requirement. */
export const OMP_SUGGESTED_PROVIDER_PREFIX = "fyagent-";
const PROVIDER_ID_PATTERN = /^[A-Za-z0-9._-]{1,80}$/;
const ENV_NAME_PATTERN = /^[A-Za-z_][A-Za-z0-9_]{0,127}$/;

export function isValidOmpProviderId(value: string): boolean {
  return PROVIDER_ID_PATTERN.test(value);
}

export function isValidOmpEnvName(value: string): boolean {
  return ENV_NAME_PATTERN.test(value);
}

export function hasControlCharacters(value: string): boolean {
  for (const char of value) {
    const code = char.codePointAt(0) ?? 0;
    if (code <= 0x1f || code === 0x7f) return true;
  }
  return false;
}

export function isValidOmpModelId(value: string): boolean {
  return (
    value.trim() === value &&
    value.length > 0 &&
    value.length <= 160 &&
    !hasControlCharacters(value) &&
    !value.startsWith("/") &&
    !value.endsWith("/") &&
    !value.includes("//")
  );
}

export function ompModelSelector(providerId: string, modelId: string): string {
  return `${providerId}/${modelId}`;
}
