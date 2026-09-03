import type {
  ManagedAuthAccountRemovalPreview,
  ManagedAuthLoginSessionSnapshot,
  ManagedAuthMutationResult,
  ManagedAuthOverview,
} from "@/v2/shared/features/managed-auth";

export const OPENAI_ACCOUNT_ID = `ma1:${"1".repeat(32)}`;
export const XAI_ACCOUNT_ID = `ma1:${"2".repeat(32)}`;
export const CODEX_CONNECTION_ID = `mc1:${"3".repeat(32)}`;
export const PROXY_CONNECTION_ID = `mc1:${"4".repeat(32)}`;
export const GROK_CONNECTION_ID = `mc1:${"5".repeat(32)}`;
export const OPENCODE_CONNECTION_ID = `mc1:${"6".repeat(32)}`;
export const ACCOUNT_REVISION = `mr1:${"a".repeat(64)}`;
export const CONNECTION_REVISION = `mr1:${"b".repeat(64)}`;
export const SESSION_ID = "123e4567-e89b-42d3-a456-426614174000";
export const OPERATION_ID = "223e4567-e89b-42d3-a456-426614174000";
export const PREVIEW_ID = `mp1:${"7".repeat(32)}`;

export function managedAuthOverviewFixture(): ManagedAuthOverview {
  return {
    contractVersion: 1,
    checkedAt: "2026-09-03T08:00:00Z",
    providers: [
      {
        provider: "openai",
        available: true,
        loginMethods: ["browser_loopback", "device_code"],
        consumers: ["codex", "opencode", "fyagent_proxy"],
        reasonCodes: [],
      },
      {
        provider: "xai",
        available: true,
        loginMethods: ["device_code"],
        consumers: ["grokbuild", "opencode", "fyagent_proxy"],
        reasonCodes: [],
      },
      {
        provider: "github_copilot",
        available: true,
        loginMethods: ["device_code"],
        consumers: ["opencode", "fyagent_proxy"],
        reasonCodes: [],
      },
    ],
    accounts: [
      {
        accountId: OPENAI_ACCOUNT_ID,
        revision: ACCOUNT_REVISION,
        provider: "openai",
        login: "person@example.com",
        displayName: "Personal",
        health: "ready",
        isDefault: true,
        lastAuthenticatedAt: "2026-09-03T07:30:00Z",
        connectedConsumerCount: 2,
        planSummary: "ChatGPT Plus",
        quotaSummary: "额度正常",
        allowedActions: ["reauthenticate", "refresh", "remove"],
        reasonCodes: [],
      },
      {
        accountId: XAI_ACCOUNT_ID,
        revision: `mr1:${"c".repeat(64)}`,
        provider: "xai",
        login: "xai@example.com",
        displayName: null,
        health: "ready",
        isDefault: true,
        lastAuthenticatedAt: "2026-09-02T20:00:00Z",
        connectedConsumerCount: 2,
        planSummary: null,
        quotaSummary: null,
        allowedActions: ["reauthenticate", "refresh", "remove"],
        reasonCodes: [],
      },
    ],
    connections: [
      {
        connectionId: CODEX_CONNECTION_ID,
        revision: CONNECTION_REVISION,
        consumer: "codex",
        targetId: "target:codex:default",
        targetLabel: "Codex",
        provider: "openai",
        accountId: OPENAI_ACCOUNT_ID,
        authStatus: "connected",
        credentialManager: "codex",
        requestMode: "third_party_api",
        requestProviderLabel: "DeepSeek API",
        officialSessionPreserved: true,
        pendingRestart: false,
        allowedActions: [
          "switch_account",
          "disconnect",
          "refresh",
          "switch_to_official",
          "open_consumer",
        ],
        checkedAt: "2026-09-03T08:00:00Z",
        reasonCodes: [],
      },
      {
        connectionId: PROXY_CONNECTION_ID,
        revision: `mr1:${"d".repeat(64)}`,
        consumer: "fyagent_proxy",
        targetId: null,
        targetLabel: null,
        provider: "openai",
        accountId: OPENAI_ACCOUNT_ID,
        authStatus: "connected",
        credentialManager: "fyagent",
        requestMode: "official_subscription",
        requestProviderLabel: "OpenAI / Codex 订阅",
        officialSessionPreserved: null,
        pendingRestart: false,
        allowedActions: ["switch_account", "disconnect", "refresh"],
        checkedAt: "2026-09-03T08:00:00Z",
        reasonCodes: [],
      },
      {
        connectionId: GROK_CONNECTION_ID,
        revision: `mr1:${"e".repeat(64)}`,
        consumer: "grokbuild",
        targetId: "target:grokbuild:default",
        targetLabel: "Grok Build",
        provider: "xai",
        accountId: XAI_ACCOUNT_ID,
        authStatus: "connected",
        credentialManager: "grokbuild",
        requestMode: "official_subscription",
        requestProviderLabel: "xAI 官方账号",
        officialSessionPreserved: null,
        pendingRestart: false,
        allowedActions: ["disconnect", "refresh", "open_consumer"],
        checkedAt: "2026-09-03T08:00:00Z",
        reasonCodes: [],
      },
      {
        connectionId: OPENCODE_CONNECTION_ID,
        revision: `mr1:${"f".repeat(64)}`,
        consumer: "opencode",
        targetId: "target:opencode:default",
        targetLabel: "OpenCode Desktop",
        provider: "xai",
        accountId: XAI_ACCOUNT_ID,
        authStatus: "connected",
        credentialManager: "opencode",
        requestMode: "provider_connections",
        requestProviderLabel: "xAI Provider",
        officialSessionPreserved: null,
        pendingRestart: false,
        allowedActions: [
          "switch_account",
          "disconnect",
          "refresh",
          "open_consumer",
        ],
        checkedAt: "2026-09-03T08:00:00Z",
        reasonCodes: [],
      },
    ],
    activeSessions: [],
    reasonCodes: [],
  };
}

export function deviceLoginSessionFixture(
  overrides: Partial<ManagedAuthLoginSessionSnapshot> = {},
): ManagedAuthLoginSessionSnapshot {
  return {
    contractVersion: 1,
    sessionId: SESSION_ID,
    provider: "openai",
    purpose: "connect_consumer",
    consumer: "codex",
    method: "device_code",
    stage: "awaiting_user",
    canCancel: true,
    canRetry: false,
    canSwitchToDeviceCode: false,
    officialHost: "auth.openai.com",
    userCode: "ABCD-EFGH",
    verificationUri: "https://auth.openai.com/codex/device",
    expiresAt: "2026-09-03T08:15:00Z",
    accountId: null,
    connectionId: null,
    reasonCode: null,
    terminal: false,
    ...overrides,
  };
}

export function removalPreviewFixture(): ManagedAuthAccountRemovalPreview {
  return {
    contractVersion: 1,
    previewId: PREVIEW_ID,
    accountId: OPENAI_ACCOUNT_ID,
    expectedRevision: ACCOUNT_REVISION,
    disconnects: [
      {
        consumer: "codex",
        targetLabel: "Codex",
        requestMode: "third_party_api",
      },
      {
        consumer: "fyagent_proxy",
        targetLabel: null,
        requestMode: "official_subscription",
      },
    ],
    preserved: [
      {
        consumer: "codex",
        targetLabel: "Codex",
        requestMode: "third_party_api",
      },
    ],
    canApply: true,
    reasonCodes: [],
  };
}

export function mutationResultFixture(
  overview = managedAuthOverviewFixture(),
): ManagedAuthMutationResult {
  return {
    contractVersion: 1,
    operationId: OPERATION_ID,
    outcome: "completed",
    overview,
    pendingRestartConsumers: [],
    reasonCode: null,
  };
}
