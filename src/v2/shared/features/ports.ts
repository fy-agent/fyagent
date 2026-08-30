import type {
  JobSnapshot,
  LocalInstallStatus,
  RemoteReleaseStatus,
} from "@/shared/codex-desktop";

import type {
  AgentCatalogResult,
  AgentCatalogId,
  DiscoverableSkill,
  DiscoverableSkillsPage,
  DiscoverSkillsPageRequest,
  FeatureSettings,
  ImportSkillSelection,
  InstalledSkill,
  McpServer,
  McpServersMap,
  SkillBackupEntry,
  SkillMigrationResult,
  SkillRepo,
  SkillHubSearchResult,
  SkillUpdateInfo,
  McpTargetId,
  SkillTargetId,
  UnmanagedSkill,
  ProviderAppId,
  ProviderQuickSetupRequest,
  ProviderMutationResult,
  ProviderSummaryQueryData,
  ProviderSwitchResult,
  WorkBuddyFetchModelsRequest,
  WorkBuddyFetchModelsResult,
  WorkBuddyModelIdsResult,
  WorkBuddySaveModelsRequest,
  WorkBuddySaveModelsResult,
  WorkBuddyStatus,
  XaiManagedSummary,
  BindXaiManagedRequest,
  BindXaiManagedResult,
  ExternalAgentLaunchDestination,
  ExternalAgentLaunchResult,
  ExternalAgentRuntimeStatus,
  QoderWorkHooksSnapshot,
  SaveQoderWorkHooksRequest,
  SaveQoderWorkHooksResult,
  ExternalMcpAgentId,
  ExternalMcpValidationResult,
  TraeWorkModelRequest,
  TraeModelValidationResult,
  TraeModelProbeResult,
  CancelTraeModelProbeResult,
  TraeWorkModelIdsResult,
  FetchedModelList,
  FetchedModelRef,
  OpenCodeFetchModelsRequest,
  OpenCodeModelSnapshot,
  OpenCodeSaveModelsRequest,
  OpenCodeSaveModelsResult,
  ModelProbeRequest,
  ModelProbeResult,
  ReachabilityResult,
  DailyMemoryFileInfo,
  DailyMemorySearchResult,
  HermesMemoryKind,
  HermesMemoryLimits,
  ManagedPrompt,
  MemoryDocumentId,
  OpenClawDirectory,
  PromptAppId,
} from "./types";
import type { AgentInstallReadinessPort } from "./agent-install-readiness";
import type { AgentAuthPort } from "./agent-auth";
import type { ChangePlansPort } from "./change-plans";

export interface AgentCatalogPort {
  get(): Promise<AgentCatalogResult>;
}

export interface ExternalAgentsPort {
  getStatus(agentId: AgentCatalogId): Promise<ExternalAgentRuntimeStatus>;
  launch(
    agentId: AgentCatalogId,
    destination: ExternalAgentLaunchDestination,
  ): Promise<ExternalAgentLaunchResult>;
}

export interface QoderWorkPort {
  getHooks(): Promise<QoderWorkHooksSnapshot>;
  saveHooks(
    request: SaveQoderWorkHooksRequest,
  ): Promise<SaveQoderWorkHooksResult>;
}

export interface ExternalMcpPort {
  validate(
    agentId: ExternalMcpAgentId,
    config: Record<string, unknown>,
  ): Promise<ExternalMcpValidationResult>;
}

export interface TraeWorkPort {
  validateModelConfig(
    request: TraeWorkModelRequest,
  ): Promise<TraeModelValidationResult>;
  testModelEndpoint(
    requestId: string,
    request: TraeWorkModelRequest,
  ): Promise<TraeModelProbeResult>;
  cancelModelEndpoint(requestId: string): Promise<CancelTraeModelProbeResult>;
  getModelIds(): Promise<TraeWorkModelIdsResult>;
}

export interface CodexDesktopPort {
  getLocalStatus(): Promise<LocalInstallStatus>;
  checkLatest(force: boolean): Promise<RemoteReleaseStatus>;
  getJob(): Promise<JobSnapshot | null>;
  startInstall(expectedReleaseId: string): Promise<JobSnapshot>;
  cancelInstall(jobId: string): Promise<JobSnapshot>;
  launch(): Promise<void>;
  openLogDirectory(): Promise<void>;
  subscribeJobUpdates(
    onSnapshot: (snapshot: JobSnapshot) => void,
  ): Promise<() => void>;
}

export interface ProvidersPort {
  getSummary(app: ProviderAppId): Promise<ProviderSummaryQueryData>;
  applyQuickSetupWithResult(
    request: ProviderQuickSetupRequest,
    app: ProviderAppId,
  ): Promise<ProviderMutationResult<ProviderSwitchResult>>;
  fetchModels(baseUrl: string, apiKey: string): Promise<FetchedModelRef[]>;
  checkReachability(baseUrl: string): Promise<ReachabilityResult>;
  checkModel(request: ModelProbeRequest): Promise<ModelProbeResult>;
  bindXaiManaged(request: BindXaiManagedRequest): Promise<BindXaiManagedResult>;
}

export interface WorkBuddyPort {
  getStatus(): Promise<WorkBuddyStatus>;
  getModelIds(): Promise<WorkBuddyModelIdsResult>;
  fetchModels(
    request: WorkBuddyFetchModelsRequest,
  ): Promise<WorkBuddyFetchModelsResult>;
  getXaiManagedSummary(): Promise<XaiManagedSummary>;
  fetchXaiManagedModels(
    accountId?: string | null,
  ): Promise<WorkBuddyFetchModelsResult>;
  saveModels(
    request: WorkBuddySaveModelsRequest,
  ): Promise<WorkBuddySaveModelsResult>;
  checkReachability(baseUrl: string): Promise<ReachabilityResult>;
  checkModel(request: ModelProbeRequest): Promise<ModelProbeResult>;
}

export interface OpenCodeModelsPort {
  getSnapshot(): Promise<OpenCodeModelSnapshot>;
  fetchProviderModels(
    request: OpenCodeFetchModelsRequest,
  ): Promise<FetchedModelList>;
  saveModels(
    request: OpenCodeSaveModelsRequest,
  ): Promise<OpenCodeSaveModelsResult>;
  checkReachability(baseUrl: string): Promise<ReachabilityResult>;
  checkModel(request: ModelProbeRequest): Promise<ModelProbeResult>;
}

export interface SkillsPort {
  getInstalled(): Promise<InstalledSkill[]>;
  getBackups(): Promise<SkillBackupEntry[]>;
  deleteBackup(backupId: string): Promise<boolean>;
  install(
    skill: DiscoverableSkill,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill>;
  uninstall(id: string): Promise<{ backupPath?: string }>;
  restoreBackup(
    backupId: string,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill>;
  toggleApp(id: string, app: SkillTargetId, enabled: boolean): Promise<boolean>;
  scanUnmanaged(): Promise<UnmanagedSkill[]>;
  importFromApps(imports: ImportSkillSelection[]): Promise<InstalledSkill[]>;
  discoverPage(
    request: DiscoverSkillsPageRequest,
  ): Promise<DiscoverableSkillsPage>;
  checkUpdates(): Promise<SkillUpdateInfo[]>;
  update(id: string): Promise<InstalledSkill>;
  migrateStorage(target: "fyagent" | "unified"): Promise<SkillMigrationResult>;
  searchSkillHub(
    query: string,
    limit: number,
    offset: number,
    category?: string,
  ): Promise<SkillHubSearchResult>;
  installSkillHub(
    slug: string,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill[]>;
  getRepos(): Promise<SkillRepo[]>;
  addRepo(repo: SkillRepo): Promise<boolean>;
  removeRepo(owner: string, name: string): Promise<boolean>;
  pickZip(): Promise<string | null>;
  installFromZip(
    filePath: string,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill[]>;
}

export interface McpPort {
  getAll(): Promise<McpServersMap>;
  upsert(server: McpServer): Promise<void>;
  delete(id: string): Promise<boolean>;
  toggleApp(
    serverId: string,
    app: McpTargetId,
    enabled: boolean,
  ): Promise<void>;
  importFromApps(): Promise<number>;
}

export interface SettingsPort {
  get(): Promise<FeatureSettings>;
  save(settings: FeatureSettings): Promise<boolean>;
  openExternal(url: string): Promise<void>;
}

export interface PromptsPort {
  getAll(app: PromptAppId): Promise<ManagedPrompt[]>;
  getCurrentFileContent(app: PromptAppId): Promise<string | null>;
  upsert(app: PromptAppId, prompt: ManagedPrompt): Promise<void>;
  delete(app: PromptAppId, id: string): Promise<void>;
  enable(app: PromptAppId, id: string): Promise<void>;
  importFromFile(app: PromptAppId): Promise<string>;
}

export interface MemoryPort {
  readDocument(id: MemoryDocumentId): Promise<string | null>;
  writeDocument(id: MemoryDocumentId, content: string): Promise<void>;
  getHermesLimits(): Promise<HermesMemoryLimits>;
  setHermesEnabled(kind: HermesMemoryKind, enabled: boolean): Promise<void>;
  listDailyFiles(): Promise<DailyMemoryFileInfo[]>;
  readDailyFile(filename: string): Promise<string | null>;
  writeDailyFile(filename: string, content: string): Promise<void>;
  deleteDailyFile(filename: string): Promise<void>;
  searchDailyFiles(query: string): Promise<DailyMemorySearchResult[]>;
  openOpenClawDirectory(subdir: OpenClawDirectory): Promise<void>;
}

export interface FeaturePorts {
  catalog: AgentCatalogPort;
  agentAuth: AgentAuthPort;
  agentInstallReadiness: AgentInstallReadinessPort;
  changePlans: ChangePlansPort;
  externalAgents: ExternalAgentsPort;
  qoderwork: QoderWorkPort;
  externalMcp: ExternalMcpPort;
  traeWork: TraeWorkPort;
  codexDesktop: CodexDesktopPort;
  providers: ProvidersPort;
  workbuddy: WorkBuddyPort;
  opencodeModels: OpenCodeModelsPort;
  skills: SkillsPort;
  mcp: McpPort;
  prompts: PromptsPort;
  memory: MemoryPort;
  settings: SettingsPort;
}
