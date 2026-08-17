export type Brand<T, Name extends string> = T & { readonly __brand: Name };

export type SchemaVersionV1 = 1;
export type SecretContractVersionV1 = "secret-contract/v1";

export type SecretRef = Brand<string, "SecretRef">;
export type SecretRefDisplay = Brand<string, "SecretRefDisplay">;
export type SecretCandidateId = Brand<string, "SecretCandidateId">;
export type SecretBackendInstanceId = Brand<string, "SecretBackendInstanceId">;
export type SecretCaptureIntentId = Brand<string, "SecretCaptureIntentId">;
export type SecretRecoveryId = Brand<string, "SecretRecoveryId">;
export type ProviderDeleteImpactId = Brand<string, "ProviderDeleteImpactId">;
export type LegacySourceLocationId = Brand<string, "LegacySourceLocationId">;
export type OwnerId = Brand<string, "OwnerId">;
export type SafeDisplayText = Brand<string, "SafeDisplayText">;
export type UtcTimestamp = Brand<string, "UtcTimestamp">;
export type SecretOwnerNamespace = Brand<string, "SecretOwnerNamespace">;
export type SecretRecordRevision = Brand<number, "SecretRecordRevision">;
export type SecretBindingRevision = Brand<number, "SecretBindingRevision">;
export type SecretOwnerBindingRevision = Brand<number, "SecretOwnerBindingRevision">;
export type SecretBindingSetRevision = Brand<number, "SecretBindingSetRevision">;
export type SecretCandidateRevision = Brand<number, "SecretCandidateRevision">;
export type SecretBackendGeneration = Brand<number, "SecretBackendGeneration">;
export type ConfirmationTimeoutSeconds = Brand<number, "ConfirmationTimeoutSeconds">;
export type BindingSetDigest = Brand<string, "BindingSetDigest">;

export type SecretOwnerKind = "provider" | "agent";
export type SecretPurpose = "codexApiKey";
export type SecretSlot = "primaryApiKey";

export interface SecretOwner {
  kind: SecretOwnerKind;
  namespace: SecretOwnerNamespace;
  ownerId: OwnerId;
  slot: SecretSlot;
}

export type SecretBackendKind = "osKeyring" | "hardware";
export type SecretBackendAvailability = "available" | "unavailable";
export type SecretPresence = "present" | "missing" | "unknown";

export const SECRET_STABLE_AVAILABILITIES = [
  "ready",
  "missing",
  "locked",
  "denied",
  "stale",
  "revoked",
  "unavailable",
] as const;

export type SecretStableAvailability = (typeof SECRET_STABLE_AVAILABILITIES)[number];

export const SECRET_LOCK_SOURCES = ["fyAgentPolicy", "backend"] as const;
export type SecretLockSource = (typeof SECRET_LOCK_SOURCES)[number];

export const SECRET_REVOCATION_SOURCES = [
  "userDelete",
  "centralBackend",
  "deviceAdministration",
  "supersededByRotation",
] as const;
export type SecretRevocationSource = (typeof SECRET_REVOCATION_SOURCES)[number];

export type SecretBackendUnavailableReason =
  | "hardwareUnregistered"
  | "hardwareDisconnected"
  | "osStoreUnavailable"
  | "centralServiceUnavailable";

export type SecretBackendOperation =
  | "captureVerify"
  | "validate"
  | "resolveForApply"
  | "delete"
  | "revoke";

export type HardwarePromptKey = "secret.hardware.confirmTouch";

export interface SecretDeviceDisplay {
  displayName: SafeDisplayText;
  deviceClass: "osAccount" | "securityKey" | "secureElement" | "unknown";
  transport: "platform" | "usb" | "nfc" | "ble" | "unknown";
}

export interface SecretBackendInstanceView {
  kind: SecretBackendKind;
  instanceId: SecretBackendInstanceId;
  generation: SecretBackendGeneration;
  availability: SecretBackendAvailability;
  device?: SecretDeviceDisplay;
}

export interface SecretBindingSetCas {
  revision: SecretBindingSetRevision;
  digest: BindingSetDigest;
  count: number;
}

export interface SecretLockView {
  source: SecretLockSource;
  lockedAt: UtcTimestamp;
}

export interface SecretRevocationView {
  source: SecretRevocationSource;
  revokedAt: UtcTimestamp;
}

export const SECRET_USER_ACTIONS = [
  "none",
  "retryCapture",
  "retryRotation",
  "retryProxyRequest",
  "retryUsageProbe",
  "retryCodingPlanUsageProbe",
  "retryModelFetch",
  "unlockFyAgent",
  "unlockBackend",
  "requestPermission",
  "captureReplacement",
  "chooseBackend",
  "confirmDevice",
  "refreshSummary",
  "refreshDeleteImpact",
  "refreshRecoveryImpact",
  "reopenChangePlan",
  "resolveLegacyConflict",
  "discardCandidate",
  "completeRecovery",
  "resumeStagedImportCutover",
  "reconnectDevice",
  "openBackendSettings",
  "contactAdministrator",
] as const;

export type SecretUserAction = (typeof SECRET_USER_ACTIONS)[number];

export type SecretErrorCode =
  | "SECRET_MISSING"
  | "SECRET_LOCKED"
  | "SECRET_PERMISSION_DENIED"
  | "SECRET_BACKEND_UNAVAILABLE"
  | "SECRET_STALE"
  | "SECRET_REVOKED"
  | "SECRET_DEVICE_MISMATCH"
  | "SECRET_OPERATION_RECOVERY_REQUIRED";

export type SecretRecoveryKind =
  | "activationCleanup"
  | "captureCompensation"
  | "deleteFinalization"
  | "ownerDetachFinalization";

export interface SecretIssueView {
  code: SecretErrorCode;
  retryable: boolean;
  action: SecretUserAction;
  lockSource?: SecretLockSource;
  revocationSource?: SecretRevocationSource;
  backendUnavailableReason?: SecretBackendUnavailableReason;
}

export interface SecretOwnerBindingSummary {
  owner: SecretOwner;
  purpose: SecretPurpose;
  bindingRevision: SecretBindingRevision;
  createdAt: UtcTimestamp;
  updatedAt: UtcTimestamp;
}

export interface SecretRefAggregate {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
  secretRefDisplay: SecretRefDisplay;
  purpose: SecretPurpose;
  presence: SecretPresence;
  availability: SecretStableAvailability;
  backend?: SecretBackendInstanceView;
  lock?: SecretLockView;
  revocation?: SecretRevocationView;
  issue?: SecretIssueView;
  createdAt: UtcTimestamp;
}

export type LegacySourceCategory =
  | "providerAuthJson"
  | "providerConfigTomlTopLevel"
  | "providerConfigTomlActiveTable"
  | "providerConfigTomlInactiveTable"
  | "providerConfigTomlInlineTable"
  | "providerUsageScriptApiKey"
  | "providerNonCanonicalProxyAlias";

export type LegacySourceOrigin =
  | "providerRow"
  | "liveAuth"
  | "liveConfig"
  | "sqlImportStaging"
  | "dbRestoreStaging"
  | "syncDownloadStaging";

export interface LegacySourceRef {
  locationId: LegacySourceLocationId;
  category: LegacySourceCategory;
  origin: LegacySourceOrigin;
}

export type CurrentScrubbableLegacySourceCoverageView =
  | { state: "none"; sourceCount: 0; categories: readonly [] }
  | {
      state: "currentSourcesPresent";
      sourceCount: number;
      categories: readonly [LegacySourceCategory, ...LegacySourceCategory[]];
    };

export type AdjacentBlockedLegacySourceCoverageView = {
  state: "none";
  observationCount: 0;
  observations: readonly [];
};

export type LegacySourceCoverageView =
  | {
      state: "clear";
      currentScrubbable: Extract<
        CurrentScrubbableLegacySourceCoverageView,
        { state: "none" }
      >;
      adjacentBlocked: AdjacentBlockedLegacySourceCoverageView;
    }
  | {
      state: "blockingSourcesPresent";
      currentScrubbable: Extract<
        CurrentScrubbableLegacySourceCoverageView,
        { state: "currentSourcesPresent" }
      >;
      adjacentBlocked: AdjacentBlockedLegacySourceCoverageView;
    };

export type LegacyOwnerState =
  | "singleValuePending"
  | "sourcesConflict"
  | "sourceInvalid"
  | "bindingComparisonPending"
  | "bindingConflict"
  | "approvalRequired";

export interface BoundOwnerBindingState {
  state: "bound";
  secretRef: SecretRef;
  secretRefDisplay: SecretRefDisplay;
  bindingRevision: SecretBindingRevision;
}

export interface LegacyOwnerBindingState {
  state: "legacy";
  legacyState: LegacyOwnerState;
  sources: readonly LegacySourceRef[];
  sourceCount: number;
  action: SecretUserAction;
}

export interface UnboundOwnerBindingState {
  state: "unbound";
}

export type OwnerBindingState =
  | BoundOwnerBindingState
  | LegacyOwnerBindingState
  | UnboundOwnerBindingState;

export interface SecretOwnerCredentialSummary {
  schemaVersion: SchemaVersionV1;
  owner: SecretOwner;
  purpose: SecretPurpose;
  ownerBindingRevision: SecretOwnerBindingRevision;
  bindingState: OwnerBindingState;
  legacySourceCoverage: LegacySourceCoverageView;
}

export type BeginCaptureIntent = "newBinding" | "replaceBinding" | "legacyReconcile";

export type SecretCaptureBindingView =
  | { state: "unbound" }
  | {
      state: "bound";
      secretRefDisplay: SecretRefDisplay;
      bindingRevision: SecretBindingRevision;
    }
  | {
      state: "legacy";
      legacyState: LegacyOwnerState;
      sourceCount: number;
    };

export interface SecretCaptureIntentView {
  schemaVersion: SchemaVersionV1;
  captureIntentId: SecretCaptureIntentId;
  owner: SecretOwner;
  purpose: SecretPurpose;
  intent: BeginCaptureIntent;
  currentBinding: SecretCaptureBindingView;
  legacySourceCoverage: LegacySourceCoverageView;
  expiresAt: UtcTimestamp;
}

export interface SecretBackendOption {
  backend: SecretBackendInstanceView;
}

export type SecretCandidateKind =
  | "newBinding"
  | "replaceBinding"
  | "rotateBindingSet"
  | "legacyReconcile"
  | "legacyScrubExistingBinding";

export type LegacyActivationComparisonPolicy =
  | "candidateEquality"
  | "explicitReplacement";

export type LegacyActivationComparisonImpact =
  | {
      policy: "candidateEquality";
      userMeaning: "verifySameValueMigration";
    }
  | {
      policy: "explicitReplacement";
      userMeaning: "replaceExistingCredential";
      affectedSourceCount: number;
      replacesBoundBinding: boolean;
    };

export type SecretCandidateState =
  | "verifiedPendingPlan"
  | "activated"
  | "discarded"
  | "cleanupRequired"
  | "expired";

export type CandidateTerminalState = "discarded" | "expired";

export interface SecretCandidateSummary {
  schemaVersion: SchemaVersionV1;
  candidateId: SecretCandidateId;
  candidateRevision: SecretCandidateRevision;
  kind: SecretCandidateKind;
  comparisonPolicy: LegacyActivationComparisonPolicy;
  comparisonImpact: LegacyActivationComparisonImpact;
  state: SecretCandidateState;
  secretRefDisplay: SecretRefDisplay;
  purpose: SecretPurpose;
  targetOwners: readonly SecretOwner[];
  legacySourceCounts: readonly {
    category: LegacySourceCategory;
    count: number;
  }[];
  createdAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
  pendingTerminalDisposition?: CandidateTerminalState;
  issue?: SecretIssueView;
}

export interface SecretMutationImpact {
  schemaVersion: SchemaVersionV1;
  secretRefDisplay: SecretRefDisplay;
  bindingSetCas: SecretBindingSetCas;
  affectedOwners: readonly SecretOwnerBindingSummary[];
  effect: "allBindingsAffected" | "oneBindingAffected";
  noFallback: true;
}

export type SecretDeleteReadiness =
  | { status: "ready" }
  | {
      status: "confirmationRequired";
      confirmation: SecretConfirmationRequirementView;
    }
  | { status: "blocked"; error: SecretIssueView };

export interface SecretDeleteImpact {
  impact: SecretMutationImpact;
  readiness: SecretDeleteReadiness;
}

export type ProviderDeleteExistingBindingView =
  | {
      state: "bound";
      secretRefDisplay: SecretRefDisplay;
      remainingOwners: readonly SecretOwner[];
      becomesOrphan: boolean;
    }
  | {
      state: "unbound";
      remainingOwners: readonly [];
      becomesOrphan: false;
    };

export type ProviderDeleteReadyImpact = {
  bindingState: "bound";
  providerDeleteImpactId: ProviderDeleteImpactId;
  owner: SecretOwner;
  existingBinding: Extract<ProviderDeleteExistingBindingView, { state: "bound" }>;
  legacySourceCoverage: Extract<LegacySourceCoverageView, { state: "clear" }>;
  deleteAllowed: true;
  effect: "none";
  secretRetained: true;
  separateSecretDeleteAction: "get_secret_delete_impact";
};

export type ProviderDeleteBlockedLegacyImpact = {
  bindingState: "legacy";
  owner: SecretOwner;
  existingBinding: Extract<ProviderDeleteExistingBindingView, { state: "bound" }>;
  legacySourceCoverage: Extract<
    LegacySourceCoverageView,
    { state: "blockingSourcesPresent" }
  >;
  deleteAllowed: false;
  effect: "none";
  action: "resolveLegacyConflict";
};

export type CodexProviderDeleteImpactDto =
  | {
      schemaVersion: SchemaVersionV1;
      status: "ready";
      impact: ProviderDeleteReadyImpact;
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "blockedLegacyResolutionRequired";
      blocked: ProviderDeleteBlockedLegacyImpact;
    };

export interface SecretConfirmationRequirementView {
  operation: SecretBackendOperation;
  device: SecretDeviceDisplay;
  timeoutSeconds: ConfirmationTimeoutSeconds;
  promptKey: HardwarePromptKey;
}

export interface BeginSecretCaptureRequest {
  captureIntentId: SecretCaptureIntentId;
  backendInstanceId: SecretBackendInstanceId;
}

export const BEGIN_SECRET_CAPTURE_REQUEST_KEYS = [
  "captureIntentId",
  "backendInstanceId",
] as const;

export interface CredentialsSnapshot {
  schemaVersion: SchemaVersionV1;
  owners: readonly SecretOwnerCredentialSummary[];
  refs: readonly SecretRefAggregate[];
  candidates: readonly SecretCandidateSummary[];
  captureIntent: SecretCaptureIntentView;
  registeredBackends: readonly SecretBackendOption[];
  secretDeleteImpact: SecretDeleteImpact;
  providerDeleteReady: Extract<
    CodexProviderDeleteImpactDto,
    { status: "ready" }
  >;
  providerDeleteBlocked: Extract<
    CodexProviderDeleteImpactDto,
    { status: "blockedLegacyResolutionRequired" }
  >;
  hardwareConfirmation: SecretConfirmationRequirementView;
  ownerDisplayNames: Readonly<Record<string, SafeDisplayText>>;
}

export const FORBIDDEN_SEMANTIC_FIELDS_V1: ReadonlySet<string> = new Set([
  "secret",
  "secretvalue",
  "value",
  "apikey",
  "openaiapikey",
  "experimentalbearertoken",
  "token",
  "accesstoken",
  "refreshtoken",
  "authorization",
  "accesskey",
  "secretkey",
  "password",
  "credential",
  "privatekey",
  "credentialblob",
  "backendlocator",
  "rawerror",
  "rawmessage",
  "rawconfig",
  "providersettings",
  "livesettings",
  "absolutepath",
  "materialdigest",
]);

export const CREDENTIAL_PREFIX_MARKERS_V1 = [
  "sk-",
  "ghp_",
  "github_pat_",
  "glpat-",
  "akia",
  "aiza",
  "ya29.",
  "npm_",
  "pypi-",
  "hf_",
  "xoxb-",
  "xoxp-",
  "xoxa-",
  "eyj",
  "bearer ",
  "bearer%20",
] as const;

export const SECRET_USER_ACTION_LABELS_ZH: Record<SecretUserAction, string> = {
  none: "无",
  retryCapture: "重新采集",
  retryRotation: "重新轮换",
  retryProxyRequest: "重新发起代理请求",
  retryUsageProbe: "重新探测用量",
  retryCodingPlanUsageProbe: "重新探测编码套餐",
  retryModelFetch: "重新拉取模型",
  unlockFyAgent: "解锁 FyAgent",
  unlockBackend: "到系统解锁",
  requestPermission: "申请权限",
  captureReplacement: "重新采集",
  chooseBackend: "采集凭据",
  confirmDevice: "确认设备",
  refreshSummary: "刷新摘要",
  refreshDeleteImpact: "刷新删除影响",
  refreshRecoveryImpact: "刷新恢复影响",
  reopenChangePlan: "打开变更计划",
  resolveLegacyConflict: "处理明文冲突",
  discardCandidate: "丢弃候选",
  completeRecovery: "完成恢复",
  resumeStagedImportCutover: "恢复导入切换",
  reconnectDevice: "重新连接设备",
  openBackendSettings: "打开后端设置",
  contactAdministrator: "联系管理员",
};

export const BINDING_STATE_LABELS_ZH: Record<OwnerBindingState["state"], string> =
  {
    bound: "已绑定",
    legacy: "明文待处理",
    unbound: "未绑定",
  };

export const AVAILABILITY_LABELS_ZH: Record<SecretStableAvailability, string> = {
  ready: "可用",
  missing: "缺失",
  locked: "已锁定",
  denied: "已拒绝",
  stale: "待清理",
  revoked: "已撤销",
  unavailable: "不可用",
};

export const REVOCATION_SOURCE_LABELS_ZH: Record<SecretRevocationSource, string> =
  {
    userDelete: "用户删除",
    centralBackend: "中心撤销",
    deviceAdministration: "设备管理",
    supersededByRotation: "轮换替代",
  };

export const CANDIDATE_KIND_LABELS_ZH: Record<SecretCandidateKind, string> = {
  newBinding: "新建绑定",
  replaceBinding: "替换绑定",
  rotateBindingSet: "轮换绑定集",
  legacyReconcile: "明文调和",
  legacyScrubExistingBinding: "清理已有绑定明文",
};

export const COMPARISON_MEANING_ZH: Record<
  LegacyActivationComparisonPolicy,
  string
> = {
  candidateEquality: "核验同一凭据后迁移",
  explicitReplacement: "替换这些旧来源，不要求旧值等于新值",
};

export const BACKEND_OPERATION_LABELS_ZH: Record<SecretBackendOperation, string> =
  {
    captureVerify: "采集核验",
    validate: "校验",
    resolveForApply: "应用",
    delete: "删除",
    revoke: "撤销",
  };

export const CANDIDATE_PLAN_BANNER = "等待变更计划 · 尚未切换绑定";
export const CAPTURE_WAITING_COPY = "等待系统安全输入，应用内不会看到密钥";
export const EMPTY_CREDENTIALS_COPY = "还没有本机凭据引用";
export const LEGACY_WARNING_COPY = "存在明文来源，先处理冲突";
export const UNBOUND_STATUS_COPY = "尚未绑定本机凭据";
export const MISSING_STATUS_COPY = "凭据缺失，重新采集";
export const NO_FALLBACK_COPY = "无退路";
export const PROVIDER_RETAINED_COPY = "只卸下该 Provider，凭据保留";
export const SEPARATE_SECRET_DELETE_COPY = "单独删除凭据";
export const PENDING_BACKEND_REACHABLE_COPY = "后端条目仍可达";

export function secretRefDisplayOf(secretRef: SecretRef): SecretRefDisplay {
  return `sec_…${secretRef.slice(-4)}` as SecretRefDisplay;
}

export interface CredentialsPort {
  readonly source: "browser" | "tauri-stub";
  listWorkspace(): Promise<CredentialsSnapshot>;
  beginCapture(request: BeginSecretCaptureRequest): Promise<void>;
}
