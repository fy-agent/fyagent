import type { BatchSetupTargetId } from "./models";

export type SessionProjectionMode = "full_dialogue" | "compact_context";

export interface PackSectionCounts {
  models: number;
  customTemplates: number;
  mcpServers: number;
  prompts: number;
  skills: number;
  sessionsIncluded: number;
  sessionsExcluded: number;
}

export interface WorkspacePackExportOptions {
  includeModels: boolean;
  includeCustomTemplates: boolean;
  includeMcp: boolean;
  includePrompts: boolean;
  includeSkills: boolean;
  includeSessions: boolean;
  includeApiKeys: boolean;
  sessionMode: SessionProjectionMode;
  sessionMaxAgeDays?: number | null;
  sessionMaxCount?: number | null;
}

export interface WorkspacePackExportRequest {
  targetPath: string;
  options: WorkspacePackExportOptions;
  passphrase?: string | null;
}

export interface WorkspacePackBudgetBreakdown {
  modelsBytes: number;
  templatesBytes: number;
  mcpBytes: number;
  promptsBytes: number;
  skillsBytes: number;
  sessionsBytes: number;
  totalUncompressedBytes: number;
  estimatedCompressedBytes: number;
  softWarningBytes: number;
  hardLimitBytes: number;
  withinHardLimit: boolean;
}

export interface WorkspacePackExportPreview {
  counts: PackSectionCounts;
  apiKeyCount: number;
  breakdown: WorkspacePackBudgetBreakdown;
  warnings: string[];
}

export interface WorkspacePackExportOutcome {
  path: string;
  packId: string;
  compressedBytes: number;
  uncompressedBytes: number;
  fileSha256: string;
  hasEncryptedSecrets: boolean;
  counts: PackSectionCounts;
  warnings: string[];
}

export interface WorkspacePackManifest {
  schema: string;
  packId: string;
  exportedAt: number;
  appVersion: string;
  platform: string;
  hasEncryptedSecrets: boolean;
  sessionProjectionMode: SessionProjectionMode;
  payloadSha256: string;
  uncompressedBytes: number;
  compressedBytes: number;
  budgetLimitBytes: number;
  counts: PackSectionCounts;
  warnings: string[];
}

export interface WorkspacePackInspectRequest {
  path: string;
  passphrase?: string | null;
}

export interface WorkspacePackImportPreview {
  path: string;
  manifest: WorkspacePackManifest;
  secretsLocked: boolean;
  decryptedApiKeyTargets: BatchSetupTargetId[];
  modelTargets: BatchSetupTargetId[];
  strictSessionCount: number;
  portableSessionCount: number;
  warnings: string[];
}

export interface WorkspacePackImportRequest {
  path: string;
  passphrase?: string | null;
  importModels: boolean;
  importCustomTemplates: boolean;
  importMcp: boolean;
  importPrompts: boolean;
  importSkills: boolean;
  importSessions: boolean;
}

export interface WorkspacePackImportOutcome {
  modelsApplied: number;
  modelsSkipped: number;
  templatesImported: number;
  mcpImported: number;
  promptsImported: number;
  skillsImported: number;
  sessionsArchived: number;
  sessionArchiveDir?: string;
  nativeSessionPackagePath?: string;
  warnings: string[];
}

export interface SshHostEntry {
  alias: string;
  hostName?: string;
  user?: string;
  port?: number;
  identityFile?: string;
}

export interface SshTargetSpec {
  host: string;
  user?: string | null;
  port?: number | null;
  identityFile?: string | null;
}

export interface SshConnectionTestResult {
  ok: boolean;
  remoteHome?: string;
  message: string;
}

export type RemoteFileCategory =
  | "model_config"
  | "mcp_config"
  | "global_prompt"
  | "skill_file";

export type RemotePushFileAction = "create" | "update" | "unknown";

export interface RemotePushSelection {
  includeModels: boolean;
  includeMcp: boolean;
  includePrompts: boolean;
  includeSkills: boolean;
  modelTargets?: BatchSetupTargetId[];
}

export interface RemotePushRequest {
  target: SshTargetSpec;
  selection: RemotePushSelection;
}

export interface RemotePushFilePreview {
  remotePath: string;
  category: RemoteFileCategory;
  bytes: number;
  mode: string;
  containsSecret: boolean;
  action: RemotePushFileAction;
}

export interface RemotePushPreviewResult {
  destination: string;
  remoteHome?: string;
  reachable: boolean;
  files: RemotePushFilePreview[];
  totalBytes: number;
  warnings: string[];
}

export interface RemotePushFileOutcome {
  remotePath: string;
  ok: boolean;
  action: string;
  message: string;
}

export interface RemotePushApplyResult {
  ok: boolean;
  backupDir: string;
  outcomes: RemotePushFileOutcome[];
  warnings: string[];
}

export interface WorkspaceTransferPort {
  pickExportPath(defaultName: string): Promise<string | null>;
  pickImportFile(): Promise<string | null>;
  previewExport(
    options: WorkspacePackExportOptions,
  ): Promise<WorkspacePackExportPreview>;
  exportPack(
    request: WorkspacePackExportRequest,
  ): Promise<WorkspacePackExportOutcome>;
  inspectPack(
    request: WorkspacePackInspectRequest,
  ): Promise<WorkspacePackImportPreview>;
  applyImport(
    request: WorkspacePackImportRequest,
  ): Promise<WorkspacePackImportOutcome>;
  listSshHosts(): Promise<SshHostEntry[]>;
  testSshTarget(target: SshTargetSpec): Promise<SshConnectionTestResult>;
  previewSshPush(request: RemotePushRequest): Promise<RemotePushPreviewResult>;
  applySshPush(request: RemotePushRequest): Promise<RemotePushApplyResult>;
}
