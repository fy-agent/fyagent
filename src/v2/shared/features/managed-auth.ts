export const MANAGED_AUTH_CONTRACT_VERSION = 1 as const;

export const MANAGED_AUTH_PROVIDERS = [
  "openai",
  "xai",
  "github_copilot",
] as const;
export type ManagedAuthProvider = (typeof MANAGED_AUTH_PROVIDERS)[number];

export const MANAGED_AUTH_CONSUMERS = [
  "codex",
  "grokbuild",
  "opencode",
  "fyagent_proxy",
] as const;
export type ManagedAuthConsumer = (typeof MANAGED_AUTH_CONSUMERS)[number];

export const MANAGED_AUTH_LOGIN_METHODS = [
  "browser_loopback",
  "device_code",
] as const;
export type ManagedAuthLoginMethod =
  (typeof MANAGED_AUTH_LOGIN_METHODS)[number];

export const MANAGED_AUTH_LOGIN_PURPOSES = [
  "save_only",
  "connect_consumer",
  "reauthenticate",
] as const;
export type ManagedAuthLoginPurpose =
  (typeof MANAGED_AUTH_LOGIN_PURPOSES)[number];

export const MANAGED_AUTH_HEALTH_STATES = [
  "ready",
  "checking",
  "requires_reauth",
  "migration_blocked",
  "unavailable",
] as const;
export type ManagedAuthHealth = (typeof MANAGED_AUTH_HEALTH_STATES)[number];

export const MANAGED_AUTH_ACCOUNT_ACTIONS = [
  "reauthenticate",
  "set_default",
  "remove",
  "refresh",
] as const;
export type ManagedAuthAccountAction =
  (typeof MANAGED_AUTH_ACCOUNT_ACTIONS)[number];

export const MANAGED_AUTH_CONNECTION_STATES = [
  "connected",
  "disconnected",
  "checking",
  "requires_reauth",
  "pending_restart",
  "unavailable",
] as const;
export type ManagedAuthConnectionState =
  (typeof MANAGED_AUTH_CONNECTION_STATES)[number];

export const MANAGED_AUTH_CONNECTION_ACTIONS = [
  "connect_account",
  "switch_account",
  "disconnect",
  "refresh",
  "restart",
  "open_consumer",
  "switch_to_official",
] as const;
export type ManagedAuthConnectionAction =
  (typeof MANAGED_AUTH_CONNECTION_ACTIONS)[number];

export const MANAGED_AUTH_REQUEST_MODES = [
  "official_subscription",
  "third_party_api",
  "provider_connections",
  "none",
  "unknown",
] as const;
export type ManagedAuthRequestMode =
  (typeof MANAGED_AUTH_REQUEST_MODES)[number];

export const MANAGED_AUTH_CREDENTIAL_MANAGERS = [
  "fyagent",
  "codex",
  "grokbuild",
  "opencode",
  "unavailable",
] as const;
export type ManagedAuthCredentialManager =
  (typeof MANAGED_AUTH_CREDENTIAL_MANAGERS)[number];

export const MANAGED_AUTH_REASON_CODES = [
  "native_only",
  "observer_unavailable",
  "operation_conflict",
  "requires_reauth",
  "migration_blocked",
  "secret_unavailable",
  "connection_unavailable",
  "native_projection_unavailable",
  "target_selection_required",
  "target_changed",
  "pending_restart",
  "external_change_detected",
  "provider_not_supported",
  "callback_unavailable",
  "device_code_expired",
  "identity_mismatch",
  "partial_completion",
  "cancelled",
  "timed_out",
  "login_failed",
  "invalid_response",
] as const;
export type ManagedAuthReasonCode = (typeof MANAGED_AUTH_REASON_CODES)[number];

export const MANAGED_AUTH_LOGIN_STAGES = [
  "preparing",
  "opening_browser",
  "awaiting_user",
  "exchanging_code",
  "saving_account",
  "connecting_consumer",
  "verifying",
  "completed",
  "partial",
  "failed",
  "cancelled",
  "expired",
] as const;
export type ManagedAuthLoginStage = (typeof MANAGED_AUTH_LOGIN_STAGES)[number];

export const MANAGED_AUTH_MUTATION_OUTCOMES = [
  "completed",
  "partial",
  "failed",
] as const;
export type ManagedAuthMutationOutcome =
  (typeof MANAGED_AUTH_MUTATION_OUTCOMES)[number];

export interface ManagedAuthProviderSummary {
  provider: ManagedAuthProvider;
  available: boolean;
  loginMethods: ManagedAuthLoginMethod[];
  consumers: ManagedAuthConsumer[];
  reasonCodes: ManagedAuthReasonCode[];
}

export interface ManagedAuthAccountSummary {
  accountId: string;
  revision: string;
  provider: ManagedAuthProvider;
  login: string;
  displayName: string | null;
  health: ManagedAuthHealth;
  isDefault: boolean;
  lastAuthenticatedAt: string | null;
  connectedConsumerCount: number;
  planSummary: string | null;
  quotaSummary: string | null;
  allowedActions: ManagedAuthAccountAction[];
  reasonCodes: ManagedAuthReasonCode[];
}

export interface ManagedAuthConnectionSummary {
  connectionId: string;
  revision: string;
  consumer: ManagedAuthConsumer;
  targetId: string | null;
  targetLabel: string | null;
  provider: ManagedAuthProvider | null;
  accountId: string | null;
  authStatus: ManagedAuthConnectionState;
  credentialManager: ManagedAuthCredentialManager;
  requestMode: ManagedAuthRequestMode;
  requestProviderLabel: string | null;
  officialSessionPreserved: boolean | null;
  pendingRestart: boolean;
  allowedActions: ManagedAuthConnectionAction[];
  checkedAt: string;
  reasonCodes: ManagedAuthReasonCode[];
}

export interface ManagedAuthLoginSessionSnapshot {
  contractVersion: typeof MANAGED_AUTH_CONTRACT_VERSION;
  sessionId: string;
  provider: ManagedAuthProvider;
  purpose: ManagedAuthLoginPurpose;
  consumer: ManagedAuthConsumer | null;
  method: ManagedAuthLoginMethod;
  stage: ManagedAuthLoginStage;
  canCancel: boolean;
  canRetry: boolean;
  canSwitchToDeviceCode: boolean;
  officialHost: string;
  userCode: string | null;
  verificationUri: string | null;
  expiresAt: string | null;
  accountId: string | null;
  connectionId: string | null;
  reasonCode: ManagedAuthReasonCode | null;
  terminal: boolean;
}

export interface ManagedAuthOverview {
  contractVersion: typeof MANAGED_AUTH_CONTRACT_VERSION;
  checkedAt: string;
  providers: ManagedAuthProviderSummary[];
  accounts: ManagedAuthAccountSummary[];
  connections: ManagedAuthConnectionSummary[];
  activeSessions: ManagedAuthLoginSessionSnapshot[];
  reasonCodes: ManagedAuthReasonCode[];
}

export interface StartManagedAuthLoginRequest {
  provider: ManagedAuthProvider;
  purpose: ManagedAuthLoginPurpose;
  consumer: ManagedAuthConsumer | null;
  method: ManagedAuthLoginMethod;
  accountId: string | null;
}

export interface ManagedAuthConnectionActionRequest {
  connectionId: string;
  expectedRevision: string;
  action: ManagedAuthConnectionAction;
  accountId: string | null;
}

export interface ManagedAuthAccountRemovalImpact {
  consumer: ManagedAuthConsumer;
  targetLabel: string | null;
  requestMode: ManagedAuthRequestMode;
}

export interface ManagedAuthAccountRemovalPreview {
  contractVersion: typeof MANAGED_AUTH_CONTRACT_VERSION;
  previewId: string;
  accountId: string;
  expectedRevision: string;
  disconnects: ManagedAuthAccountRemovalImpact[];
  preserved: ManagedAuthAccountRemovalImpact[];
  canApply: boolean;
  reasonCodes: ManagedAuthReasonCode[];
}

export interface ManagedAuthMutationResult {
  contractVersion: typeof MANAGED_AUTH_CONTRACT_VERSION;
  operationId: string;
  outcome: ManagedAuthMutationOutcome;
  overview: ManagedAuthOverview;
  pendingRestartConsumers: ManagedAuthConsumer[];
  reasonCode: ManagedAuthReasonCode | null;
}

export interface ManagedAuthPort {
  getOverview(): Promise<ManagedAuthOverview>;
  startLogin(
    request: StartManagedAuthLoginRequest,
  ): Promise<ManagedAuthLoginSessionSnapshot>;
  getLoginSession(sessionId: string): Promise<ManagedAuthLoginSessionSnapshot>;
  cancelLogin(sessionId: string): Promise<ManagedAuthLoginSessionSnapshot>;
  reopenLogin(sessionId: string): Promise<ManagedAuthLoginSessionSnapshot>;
  switchLoginMethod(
    sessionId: string,
    method: ManagedAuthLoginMethod,
  ): Promise<ManagedAuthLoginSessionSnapshot>;
  setDefaultAccount(
    accountId: string,
    expectedRevision: string,
  ): Promise<ManagedAuthMutationResult>;
  previewAccountRemoval(
    accountId: string,
    expectedRevision: string,
  ): Promise<ManagedAuthAccountRemovalPreview>;
  removeAccount(
    previewId: string,
    accountId: string,
    expectedRevision: string,
  ): Promise<ManagedAuthMutationResult>;
  applyConnectionAction(
    request: ManagedAuthConnectionActionRequest,
  ): Promise<ManagedAuthMutationResult>;
}

const DATA_ERROR = "账号与认证数据不可用";
const REQUEST_ERROR = "账号与认证请求无效";
const ACCOUNT_ID_PATTERN = /^ma1:[0-9a-f]{32}$/u;
const CONNECTION_ID_PATTERN = /^mc1:[0-9a-f]{32}$/u;
const PREVIEW_ID_PATTERN = /^mp1:[0-9a-f]{32}$/u;
const REVISION_PATTERN = /^mr1:[0-9a-f]{64}$/u;
const SESSION_ID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
const OPERATION_ID_PATTERN = SESSION_ID_PATTERN;
const ISO_TIMESTAMP_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/u;

function dataError(): never {
  throw new Error(DATA_ERROR);
}

function requestError(): never {
  throw new Error(REQUEST_ERROR);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  keys: readonly string[],
): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length &&
    actual.every((key, index) => key === expected[index])
  );
}

function isOneOf<T extends string>(
  value: unknown,
  values: readonly T[],
): value is T {
  return typeof value === "string" && values.includes(value as T);
}

function parseUniqueEnumList<T extends string>(
  value: unknown,
  values: readonly T[],
): T[] {
  if (!Array.isArray(value)) dataError();
  const parsed = value.map((item) => {
    if (!isOneOf(item, values)) dataError();
    return item;
  });
  if (new Set(parsed).size !== parsed.length) dataError();
  return parsed;
}

function parseBoundedText(value: unknown, maxLength: number): string {
  if (
    typeof value !== "string" ||
    value.trim().length === 0 ||
    value.length > maxLength ||
    /[\r\n\0]/u.test(value)
  ) {
    dataError();
  }
  return value;
}

function parseNullableText(value: unknown, maxLength: number): string | null {
  return value === null ? null : parseBoundedText(value, maxLength);
}

function isIsoTimestamp(value: unknown): value is string {
  if (
    typeof value !== "string" ||
    !ISO_TIMESTAMP_PATTERN.test(value) ||
    value.length > 40
  ) {
    return false;
  }
  return Number.isFinite(Date.parse(value));
}

function parseNullableTimestamp(value: unknown): string | null {
  if (value === null) return null;
  if (!isIsoTimestamp(value)) dataError();
  return value;
}

function parseAccountId(value: unknown): string {
  if (typeof value !== "string" || !ACCOUNT_ID_PATTERN.test(value)) {
    dataError();
  }
  return value;
}

function parseNullableAccountId(value: unknown): string | null {
  return value === null ? null : parseAccountId(value);
}

function parseConnectionId(value: unknown): string {
  if (typeof value !== "string" || !CONNECTION_ID_PATTERN.test(value)) {
    dataError();
  }
  return value;
}

function parseRevision(value: unknown): string {
  if (typeof value !== "string" || !REVISION_PATTERN.test(value)) dataError();
  return value;
}

function parseSessionId(value: unknown): string {
  if (typeof value !== "string" || !SESSION_ID_PATTERN.test(value)) dataError();
  return value;
}

function parseTargetId(value: unknown): string | null {
  if (value === null) return null;
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > 160 ||
    !/^[A-Za-z0-9:._-]+$/u.test(value)
  ) {
    dataError();
  }
  return value;
}

function parseProviderSummary(value: unknown): ManagedAuthProviderSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "provider",
      "available",
      "loginMethods",
      "consumers",
      "reasonCodes",
    ]) ||
    !isOneOf(value.provider, MANAGED_AUTH_PROVIDERS) ||
    typeof value.available !== "boolean"
  ) {
    dataError();
  }
  const loginMethods = parseUniqueEnumList(
    value.loginMethods,
    MANAGED_AUTH_LOGIN_METHODS,
  );
  const consumers = parseUniqueEnumList(
    value.consumers,
    MANAGED_AUTH_CONSUMERS,
  );
  const reasonCodes = parseUniqueEnumList(
    value.reasonCodes,
    MANAGED_AUTH_REASON_CODES,
  );
  if (
    (value.available && loginMethods.length === 0) ||
    (!value.available && loginMethods.length > 0) ||
    (value.provider !== "openai" && loginMethods.includes("browser_loopback"))
  ) {
    dataError();
  }
  return {
    provider: value.provider,
    available: value.available,
    loginMethods,
    consumers,
    reasonCodes,
  };
}

function parseAccount(value: unknown): ManagedAuthAccountSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "accountId",
      "revision",
      "provider",
      "login",
      "displayName",
      "health",
      "isDefault",
      "lastAuthenticatedAt",
      "connectedConsumerCount",
      "planSummary",
      "quotaSummary",
      "allowedActions",
      "reasonCodes",
    ]) ||
    !isOneOf(value.provider, MANAGED_AUTH_PROVIDERS) ||
    !isOneOf(value.health, MANAGED_AUTH_HEALTH_STATES) ||
    typeof value.isDefault !== "boolean" ||
    !Number.isInteger(value.connectedConsumerCount) ||
    (value.connectedConsumerCount as number) < 0 ||
    (value.connectedConsumerCount as number) > MANAGED_AUTH_CONSUMERS.length
  ) {
    dataError();
  }
  return {
    accountId: parseAccountId(value.accountId),
    revision: parseRevision(value.revision),
    provider: value.provider,
    login: parseBoundedText(value.login, 160),
    displayName: parseNullableText(value.displayName, 120),
    health: value.health,
    isDefault: value.isDefault,
    lastAuthenticatedAt: parseNullableTimestamp(value.lastAuthenticatedAt),
    connectedConsumerCount: value.connectedConsumerCount as number,
    planSummary: parseNullableText(value.planSummary, 80),
    quotaSummary: parseNullableText(value.quotaSummary, 100),
    allowedActions: parseUniqueEnumList(
      value.allowedActions,
      MANAGED_AUTH_ACCOUNT_ACTIONS,
    ),
    reasonCodes: parseUniqueEnumList(
      value.reasonCodes,
      MANAGED_AUTH_REASON_CODES,
    ),
  };
}

function parseConnection(value: unknown): ManagedAuthConnectionSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "connectionId",
      "revision",
      "consumer",
      "targetId",
      "targetLabel",
      "provider",
      "accountId",
      "authStatus",
      "credentialManager",
      "requestMode",
      "requestProviderLabel",
      "officialSessionPreserved",
      "pendingRestart",
      "allowedActions",
      "checkedAt",
      "reasonCodes",
    ]) ||
    !isOneOf(value.consumer, MANAGED_AUTH_CONSUMERS) ||
    (value.provider !== null &&
      !isOneOf(value.provider, MANAGED_AUTH_PROVIDERS)) ||
    !isOneOf(value.authStatus, MANAGED_AUTH_CONNECTION_STATES) ||
    !isOneOf(value.credentialManager, MANAGED_AUTH_CREDENTIAL_MANAGERS) ||
    !isOneOf(value.requestMode, MANAGED_AUTH_REQUEST_MODES) ||
    (value.officialSessionPreserved !== null &&
      typeof value.officialSessionPreserved !== "boolean") ||
    typeof value.pendingRestart !== "boolean" ||
    !isIsoTimestamp(value.checkedAt)
  ) {
    dataError();
  }
  const targetId = parseTargetId(value.targetId);
  const targetLabel = parseNullableText(value.targetLabel, 120);
  const accountId = parseNullableAccountId(value.accountId);
  const requestProviderLabel = parseNullableText(
    value.requestProviderLabel,
    120,
  );
  if (
    (targetId === null) !== (targetLabel === null) ||
    (accountId !== null && value.provider === null) ||
    (value.pendingRestart && value.authStatus !== "pending_restart") ||
    (!value.pendingRestart && value.authStatus === "pending_restart") ||
    (value.requestMode === "none" && requestProviderLabel !== null)
  ) {
    dataError();
  }
  return {
    connectionId: parseConnectionId(value.connectionId),
    revision: parseRevision(value.revision),
    consumer: value.consumer,
    targetId,
    targetLabel,
    provider: value.provider,
    accountId,
    authStatus: value.authStatus,
    credentialManager: value.credentialManager,
    requestMode: value.requestMode,
    requestProviderLabel,
    officialSessionPreserved: value.officialSessionPreserved,
    pendingRestart: value.pendingRestart,
    allowedActions: parseUniqueEnumList(
      value.allowedActions,
      MANAGED_AUTH_CONNECTION_ACTIONS,
    ),
    checkedAt: value.checkedAt,
    reasonCodes: parseUniqueEnumList(
      value.reasonCodes,
      MANAGED_AUTH_REASON_CODES,
    ),
  };
}

const OFFICIAL_HOST: Record<ManagedAuthProvider, string> = {
  openai: "auth.openai.com",
  xai: "auth.x.ai",
  github_copilot: "github.com",
};

function parseVerificationUri(
  value: unknown,
  expectedHost: string,
): string | null {
  if (value === null) return null;
  if (typeof value !== "string" || value.length > 240) dataError();
  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch {
    dataError();
  }
  if (
    parsed.protocol !== "https:" ||
    parsed.hostname !== expectedHost ||
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.search !== "" ||
    parsed.hash !== ""
  ) {
    dataError();
  }
  return parsed.toString();
}

export function parseManagedAuthLoginSession(
  value: unknown,
): ManagedAuthLoginSessionSnapshot {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "sessionId",
      "provider",
      "purpose",
      "consumer",
      "method",
      "stage",
      "canCancel",
      "canRetry",
      "canSwitchToDeviceCode",
      "officialHost",
      "userCode",
      "verificationUri",
      "expiresAt",
      "accountId",
      "connectionId",
      "reasonCode",
      "terminal",
    ]) ||
    value.contractVersion !== MANAGED_AUTH_CONTRACT_VERSION ||
    !isOneOf(value.provider, MANAGED_AUTH_PROVIDERS) ||
    !isOneOf(value.purpose, MANAGED_AUTH_LOGIN_PURPOSES) ||
    (value.consumer !== null &&
      !isOneOf(value.consumer, MANAGED_AUTH_CONSUMERS)) ||
    !isOneOf(value.method, MANAGED_AUTH_LOGIN_METHODS) ||
    !isOneOf(value.stage, MANAGED_AUTH_LOGIN_STAGES) ||
    typeof value.canCancel !== "boolean" ||
    typeof value.canRetry !== "boolean" ||
    typeof value.canSwitchToDeviceCode !== "boolean" ||
    typeof value.officialHost !== "string" ||
    typeof value.terminal !== "boolean" ||
    (value.reasonCode !== null &&
      !isOneOf(value.reasonCode, MANAGED_AUTH_REASON_CODES))
  ) {
    dataError();
  }
  if (value.officialHost !== OFFICIAL_HOST[value.provider]) dataError();
  const accountId = parseNullableAccountId(value.accountId);
  const connectionId =
    value.connectionId === null ? null : parseConnectionId(value.connectionId);
  const expiresAt = parseNullableTimestamp(value.expiresAt);
  const verificationUri = parseVerificationUri(
    value.verificationUri,
    value.officialHost,
  );
  const userCode =
    value.userCode === null
      ? null
      : typeof value.userCode === "string" &&
          /^[A-Z0-9-]{4,32}$/u.test(value.userCode)
        ? value.userCode
        : dataError();
  const terminalStages: ManagedAuthLoginStage[] = [
    "completed",
    "partial",
    "failed",
    "cancelled",
    "expired",
  ];
  const terminal = terminalStages.includes(value.stage);
  const retryStages: ManagedAuthLoginStage[] = ["failed", "expired"];
  if (
    value.terminal !== terminal ||
    value.canCancel === terminal ||
    value.canRetry !== retryStages.includes(value.stage) ||
    value.canSwitchToDeviceCode !==
      (value.provider === "openai" &&
        value.method === "browser_loopback" &&
        !terminal) ||
    (value.method === "browser_loopback" &&
      (userCode !== null || verificationUri !== null || expiresAt !== null)) ||
    (value.method === "device_code" &&
      !terminal &&
      (userCode === null || verificationUri === null || expiresAt === null)) ||
    (value.purpose === "save_only" && value.consumer !== null) ||
    (value.purpose === "connect_consumer" && value.consumer === null) ||
    (value.purpose === "reauthenticate" && accountId === null) ||
    (value.stage === "completed" &&
      (accountId === null ||
        (value.reasonCode !== null &&
          value.reasonCode !== "pending_restart"))) ||
    (value.stage === "partial" && accountId === null) ||
    ((["failed", "cancelled", "expired"] as ManagedAuthLoginStage[]).includes(
      value.stage,
    ) &&
      value.reasonCode === null)
  ) {
    dataError();
  }
  return {
    contractVersion: MANAGED_AUTH_CONTRACT_VERSION,
    sessionId: parseSessionId(value.sessionId),
    provider: value.provider,
    purpose: value.purpose,
    consumer: value.consumer,
    method: value.method,
    stage: value.stage,
    canCancel: value.canCancel,
    canRetry: value.canRetry,
    canSwitchToDeviceCode: value.canSwitchToDeviceCode,
    officialHost: value.officialHost,
    userCode,
    verificationUri,
    expiresAt,
    accountId,
    connectionId,
    reasonCode: value.reasonCode,
    terminal: value.terminal,
  };
}

export function parseManagedAuthOverview(value: unknown): ManagedAuthOverview {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "checkedAt",
      "providers",
      "accounts",
      "connections",
      "activeSessions",
      "reasonCodes",
    ]) ||
    value.contractVersion !== MANAGED_AUTH_CONTRACT_VERSION ||
    !isIsoTimestamp(value.checkedAt) ||
    !Array.isArray(value.providers) ||
    value.providers.length > MANAGED_AUTH_PROVIDERS.length ||
    !Array.isArray(value.accounts) ||
    value.accounts.length > 64 ||
    !Array.isArray(value.connections) ||
    value.connections.length > 128 ||
    !Array.isArray(value.activeSessions) ||
    value.activeSessions.length > 8
  ) {
    dataError();
  }
  const providers = value.providers.map(parseProviderSummary);
  const accounts = value.accounts.map(parseAccount);
  const connections = value.connections.map(parseConnection);
  const activeSessions = value.activeSessions.map(parseManagedAuthLoginSession);
  const reasonCodes = parseUniqueEnumList(
    value.reasonCodes,
    MANAGED_AUTH_REASON_CODES,
  );
  const unique = (items: string[]) => new Set(items).size === items.length;
  if (
    !unique(providers.map((item) => item.provider)) ||
    !unique(accounts.map((item) => item.accountId)) ||
    !unique(connections.map((item) => item.connectionId)) ||
    !unique(activeSessions.map((item) => item.sessionId))
  ) {
    dataError();
  }
  const providerSet = new Set(providers.map((item) => item.provider));
  const accountMap = new Map(accounts.map((item) => [item.accountId, item]));
  if (accounts.some((account) => !providerSet.has(account.provider)))
    dataError();
  for (const connection of connections) {
    if (connection.accountId === null) continue;
    const account = accountMap.get(connection.accountId);
    if (!account || account.provider !== connection.provider) dataError();
  }
  for (const account of accounts) {
    const consumers = new Set(
      connections
        .filter((connection) => connection.accountId === account.accountId)
        .map((connection) => connection.consumer),
    );
    if (consumers.size !== account.connectedConsumerCount) dataError();
  }
  for (const session of activeSessions) {
    if (session.accountId !== null && !accountMap.has(session.accountId)) {
      dataError();
    }
  }
  return {
    contractVersion: MANAGED_AUTH_CONTRACT_VERSION,
    checkedAt: value.checkedAt,
    providers,
    accounts,
    connections,
    activeSessions,
    reasonCodes,
  };
}

function parseRemovalImpact(value: unknown): ManagedAuthAccountRemovalImpact {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["consumer", "targetLabel", "requestMode"]) ||
    !isOneOf(value.consumer, MANAGED_AUTH_CONSUMERS) ||
    !isOneOf(value.requestMode, MANAGED_AUTH_REQUEST_MODES)
  ) {
    dataError();
  }
  return {
    consumer: value.consumer,
    targetLabel: parseNullableText(value.targetLabel, 120),
    requestMode: value.requestMode,
  };
}

export function parseManagedAuthRemovalPreview(
  value: unknown,
): ManagedAuthAccountRemovalPreview {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "previewId",
      "accountId",
      "expectedRevision",
      "disconnects",
      "preserved",
      "canApply",
      "reasonCodes",
    ]) ||
    value.contractVersion !== MANAGED_AUTH_CONTRACT_VERSION ||
    typeof value.previewId !== "string" ||
    !PREVIEW_ID_PATTERN.test(value.previewId) ||
    !Array.isArray(value.disconnects) ||
    value.disconnects.length > 16 ||
    !Array.isArray(value.preserved) ||
    value.preserved.length > 16 ||
    typeof value.canApply !== "boolean"
  ) {
    dataError();
  }
  return {
    contractVersion: MANAGED_AUTH_CONTRACT_VERSION,
    previewId: value.previewId,
    accountId: parseAccountId(value.accountId),
    expectedRevision: parseRevision(value.expectedRevision),
    disconnects: value.disconnects.map(parseRemovalImpact),
    preserved: value.preserved.map(parseRemovalImpact),
    canApply: value.canApply,
    reasonCodes: parseUniqueEnumList(
      value.reasonCodes,
      MANAGED_AUTH_REASON_CODES,
    ),
  };
}

export function parseManagedAuthMutationResult(
  value: unknown,
): ManagedAuthMutationResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "operationId",
      "outcome",
      "overview",
      "pendingRestartConsumers",
      "reasonCode",
    ]) ||
    value.contractVersion !== MANAGED_AUTH_CONTRACT_VERSION ||
    typeof value.operationId !== "string" ||
    !OPERATION_ID_PATTERN.test(value.operationId) ||
    !isOneOf(value.outcome, MANAGED_AUTH_MUTATION_OUTCOMES) ||
    (value.reasonCode !== null &&
      !isOneOf(value.reasonCode, MANAGED_AUTH_REASON_CODES))
  ) {
    dataError();
  }
  const pendingRestartConsumers = parseUniqueEnumList(
    value.pendingRestartConsumers,
    MANAGED_AUTH_CONSUMERS,
  );
  if (
    (value.outcome === "completed" &&
      value.reasonCode !== null &&
      value.reasonCode !== "pending_restart") ||
    (value.outcome !== "completed" && value.reasonCode === null)
  ) {
    dataError();
  }
  return {
    contractVersion: MANAGED_AUTH_CONTRACT_VERSION,
    operationId: value.operationId,
    outcome: value.outcome,
    overview: parseManagedAuthOverview(value.overview),
    pendingRestartConsumers,
    reasonCode: value.reasonCode,
  };
}

function requestRecord(value: unknown, keys: readonly string[]) {
  if (!isRecord(value) || !hasExactKeys(value, keys)) requestError();
  return value;
}

function requestAccountId(value: unknown): string {
  if (typeof value !== "string" || !ACCOUNT_ID_PATTERN.test(value)) {
    requestError();
  }
  return value;
}

function requestRevision(value: unknown): string {
  if (typeof value !== "string" || !REVISION_PATTERN.test(value)) {
    requestError();
  }
  return value;
}

function requestSessionId(value: unknown): string {
  if (typeof value !== "string" || !SESSION_ID_PATTERN.test(value)) {
    requestError();
  }
  return value;
}

export function assertStartManagedAuthLoginRequest(
  value: StartManagedAuthLoginRequest,
): StartManagedAuthLoginRequest {
  const request = requestRecord(value, [
    "provider",
    "purpose",
    "consumer",
    "method",
    "accountId",
  ]);
  if (
    !isOneOf(request.provider, MANAGED_AUTH_PROVIDERS) ||
    !isOneOf(request.purpose, MANAGED_AUTH_LOGIN_PURPOSES) ||
    (request.consumer !== null &&
      !isOneOf(request.consumer, MANAGED_AUTH_CONSUMERS)) ||
    !isOneOf(request.method, MANAGED_AUTH_LOGIN_METHODS) ||
    (request.accountId !== null &&
      (typeof request.accountId !== "string" ||
        !ACCOUNT_ID_PATTERN.test(request.accountId))) ||
    (request.provider !== "openai" && request.method === "browser_loopback") ||
    (request.purpose === "save_only" && request.consumer !== null) ||
    (request.purpose === "connect_consumer" &&
      (request.consumer === null || request.accountId !== null)) ||
    (request.purpose === "reauthenticate" && request.accountId === null)
  ) {
    requestError();
  }
  return {
    provider: request.provider,
    purpose: request.purpose,
    consumer: request.consumer,
    method: request.method,
    accountId: request.accountId,
  };
}

export function assertManagedAuthConnectionActionRequest(
  value: ManagedAuthConnectionActionRequest,
): ManagedAuthConnectionActionRequest {
  const request = requestRecord(value, [
    "connectionId",
    "expectedRevision",
    "action",
    "accountId",
  ]);
  if (
    typeof request.connectionId !== "string" ||
    !CONNECTION_ID_PATTERN.test(request.connectionId) ||
    typeof request.expectedRevision !== "string" ||
    !REVISION_PATTERN.test(request.expectedRevision) ||
    !isOneOf(request.action, MANAGED_AUTH_CONNECTION_ACTIONS) ||
    (request.accountId !== null &&
      (typeof request.accountId !== "string" ||
        !ACCOUNT_ID_PATTERN.test(request.accountId))) ||
    (
      ["connect_account", "switch_account"] as ManagedAuthConnectionAction[]
    ).includes(request.action) !==
      (request.accountId !== null)
  ) {
    requestError();
  }
  return {
    connectionId: request.connectionId,
    expectedRevision: request.expectedRevision,
    action: request.action,
    accountId: request.accountId,
  };
}

export function assertManagedAuthAccountMutation(
  accountId: string,
  expectedRevision: string,
): { accountId: string; expectedRevision: string } {
  return {
    accountId: requestAccountId(accountId),
    expectedRevision: requestRevision(expectedRevision),
  };
}

export function assertManagedAuthRemovalMutation(
  previewId: string,
  accountId: string,
  expectedRevision: string,
): { previewId: string; accountId: string; expectedRevision: string } {
  if (!PREVIEW_ID_PATTERN.test(previewId)) requestError();
  return {
    previewId,
    accountId: requestAccountId(accountId),
    expectedRevision: requestRevision(expectedRevision),
  };
}

export function assertManagedAuthSessionId(sessionId: string): string {
  return requestSessionId(sessionId);
}

export function assertManagedAuthLoginMethod(
  method: ManagedAuthLoginMethod,
): ManagedAuthLoginMethod {
  if (!MANAGED_AUTH_LOGIN_METHODS.includes(method)) requestError();
  return method;
}

export function parseManagedAuthCommandError(
  value: unknown,
): ManagedAuthReasonCode | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["contractVersion", "reasonCode"]) ||
    value.contractVersion !== MANAGED_AUTH_CONTRACT_VERSION ||
    !isOneOf(value.reasonCode, MANAGED_AUTH_REASON_CODES)
  ) {
    return null;
  }
  return value.reasonCode;
}
