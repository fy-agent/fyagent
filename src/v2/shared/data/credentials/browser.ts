import { decodeBeginSecretCaptureRequest, decodeCredentialsSnapshot } from "./decoder";
import type {
  BeginSecretCaptureRequest,
  CredentialsPort,
  CredentialsSnapshot,
  LegacySourceCoverageView,
  SecretBackendInstanceView,
  SecretOwner,
  SecretRef,
  SecretRefDisplay,
} from "./types";
import { secretRefDisplayOf } from "./types";

const STAMP = "2026-08-18T00:00:00.000Z";
const EXPIRES = "2026-12-31T23:59:59.000Z";
const DIGEST = "11".repeat(32);

function hexId(prefix: string, nibble: string, last4: string): string {
  return `${prefix}_${nibble.repeat(12)}4${nibble.repeat(3)}8${nibble.repeat(11)}${last4}`;
}

const REF = {
  ready: hexId("sec", "1", "ab12") as SecretRef,
  locked: hexId("sec", "2", "cd34") as SecretRef,
  missing: hexId("sec", "3", "ef56") as SecretRef,
  revoked: hexId("sec", "4", "aa78") as SecretRef,
  unavailable: hexId("sec", "5", "bb90") as SecretRef,
  candidate: hexId("sec", "6", "cc01") as SecretRef,
  discard: hexId("sec", "7", "dd23") as SecretRef,
  expired: hexId("sec", "8", "ee45") as SecretRef,
  cleanup: hexId("sec", "9", "ff67") as SecretRef,
  denied: hexId("sec", "a", "a1b2") as SecretRef,
  stale: hexId("sec", "b", "c3d4") as SecretRef,
  backendLocked: hexId("sec", "c", "d5e6") as SecretRef,
};

const DISPLAY: Record<keyof typeof REF, SecretRefDisplay> = {
  ready: secretRefDisplayOf(REF.ready),
  locked: secretRefDisplayOf(REF.locked),
  missing: secretRefDisplayOf(REF.missing),
  revoked: secretRefDisplayOf(REF.revoked),
  unavailable: secretRefDisplayOf(REF.unavailable),
  candidate: secretRefDisplayOf(REF.candidate),
  discard: secretRefDisplayOf(REF.discard),
  expired: secretRefDisplayOf(REF.expired),
  cleanup: secretRefDisplayOf(REF.cleanup),
  denied: secretRefDisplayOf(REF.denied),
  stale: secretRefDisplayOf(REF.stale),
  backendLocked: secretRefDisplayOf(REF.backendLocked),
};

function owner(ownerId: string): SecretOwner {
  return {
    kind: "provider",
    namespace: "codex" as SecretOwner["namespace"],
    ownerId: ownerId as SecretOwner["ownerId"],
    slot: "primaryApiKey",
  };
}

const clearCoverage: LegacySourceCoverageView = {
  state: "clear",
  currentScrubbable: { state: "none", sourceCount: 0, categories: [] },
  adjacentBlocked: { state: "none", observationCount: 0, observations: [] },
};

const blockingCoverage: LegacySourceCoverageView = {
  state: "blockingSourcesPresent",
  currentScrubbable: {
    state: "currentSourcesPresent",
    sourceCount: 2,
    categories: ["providerAuthJson", "providerConfigTomlTopLevel"],
  },
  adjacentBlocked: { state: "none", observationCount: 0, observations: [] },
};

const osKeyring: SecretBackendInstanceView = {
  kind: "osKeyring",
  instanceId: hexId("sbi", "c", "1001") as SecretBackendInstanceView["instanceId"],
  generation: 1 as SecretBackendInstanceView["generation"],
  availability: "available",
  device: {
    displayName: "本机钥匙串" as never,
    deviceClass: "osAccount",
    transport: "platform",
  },
};

function boundSummary(
  ownerId: string,
  secretRef: SecretRef,
  display: SecretRefDisplay,
) {
  return {
    schemaVersion: 1 as const,
    owner: owner(ownerId),
    purpose: "codexApiKey" as const,
    ownerBindingRevision: 1 as never,
    bindingState: {
      state: "bound" as const,
      secretRef,
      secretRefDisplay: display,
      bindingRevision: 1 as never,
    },
    legacySourceCoverage: clearCoverage,
  };
}

function refAggregate(
  secretRef: SecretRef,
  display: SecretRefDisplay,
  availability: CredentialsSnapshot["refs"][number]["availability"],
  extra: Partial<CredentialsSnapshot["refs"][number]> = {},
) {
  return {
    schemaVersion: 1 as const,
    secretRef,
    secretRefDisplay: display,
    purpose: "codexApiKey" as const,
    presence:
      availability === "missing" || availability === "revoked"
        ? ("missing" as const)
        : ("present" as const),
    availability,
    backend: osKeyring,
    createdAt: STAMP as never,
    ...extra,
  };
}

const OWNERS = {
  ready: "alpha-ready",
  legacy: "beta-legacy",
  unbound: "gamma-unbound",
  locked: "delta-locked",
  missing: "epsilon-missing",
  revoked: "zeta-revoked",
  unavailable: "eta-hardware",
  candidate: "theta-plan",
  discard: "iota-discard",
  expired: "kappa-expired",
  cleanup: "lambda-cleanup",
  denied: "mu-denied",
  stale: "nu-stale",
  backendLocked: "xi-syslock",
  shared: "share-reader",
} as const;

export const credentialBrowserFixtures: CredentialsSnapshot = {
  schemaVersion: 1,
  ownerDisplayNames: {
    [OWNERS.ready]: "主编码" as never,
    [OWNERS.legacy]: "明文冲突" as never,
    [OWNERS.unbound]: "空引用" as never,
    [OWNERS.locked]: "策略锁定" as never,
    [OWNERS.missing]: "缺失凭据" as never,
    [OWNERS.revoked]: "撤销项" as never,
    [OWNERS.unavailable]: "硬件未注册" as never,
    [OWNERS.candidate]: "待计划" as never,
    [OWNERS.discard]: "待丢弃" as never,
    [OWNERS.expired]: "已过期" as never,
    [OWNERS.cleanup]: "待清理" as never,
    [OWNERS.denied]: "已拒绝" as never,
    [OWNERS.stale]: "过期待清理" as never,
    [OWNERS.backendLocked]: "系统锁定" as never,
    [OWNERS.shared]: "共享只读" as never,
  },
  owners: [
    boundSummary(OWNERS.ready, REF.ready, DISPLAY.ready),
    {
      schemaVersion: 1,
      owner: owner(OWNERS.legacy),
      purpose: "codexApiKey",
      ownerBindingRevision: 1 as never,
      bindingState: {
        state: "legacy",
        legacyState: "sourcesConflict",
        sources: [
          {
            locationId: `lsl_${"1".repeat(32)}` as never,
            category: "providerAuthJson",
            origin: "liveAuth",
          },
          {
            locationId: `lsl_${"2".repeat(32)}` as never,
            category: "providerConfigTomlTopLevel",
            origin: "liveConfig",
          },
        ],
        sourceCount: 2,
        action: "resolveLegacyConflict",
      },
      legacySourceCoverage: blockingCoverage,
    },
    {
      schemaVersion: 1,
      owner: owner(OWNERS.unbound),
      purpose: "codexApiKey",
      ownerBindingRevision: 1 as never,
      bindingState: { state: "unbound" },
      legacySourceCoverage: clearCoverage,
    },
    boundSummary(OWNERS.locked, REF.locked, DISPLAY.locked),
    boundSummary(OWNERS.backendLocked, REF.backendLocked, DISPLAY.backendLocked),
    boundSummary(OWNERS.missing, REF.missing, DISPLAY.missing),
    boundSummary(OWNERS.revoked, REF.revoked, DISPLAY.revoked),
    boundSummary(OWNERS.unavailable, REF.unavailable, DISPLAY.unavailable),
    {
      schemaVersion: 1,
      owner: owner(OWNERS.candidate),
      purpose: "codexApiKey",
      ownerBindingRevision: 1 as never,
      bindingState: { state: "unbound" },
      legacySourceCoverage: clearCoverage,
    },
    boundSummary(OWNERS.discard, REF.ready, DISPLAY.ready),
    {
      schemaVersion: 1,
      owner: owner(OWNERS.expired),
      purpose: "codexApiKey",
      ownerBindingRevision: 1 as never,
      bindingState: { state: "unbound" },
      legacySourceCoverage: clearCoverage,
    },
    boundSummary(OWNERS.cleanup, REF.cleanup, DISPLAY.cleanup),
    boundSummary(OWNERS.denied, REF.denied, DISPLAY.denied),
    boundSummary(OWNERS.stale, REF.stale, DISPLAY.stale),
  ],
  refs: [
    refAggregate(REF.ready, DISPLAY.ready, "ready"),
    refAggregate(REF.locked, DISPLAY.locked, "locked", {
      lock: { source: "fyAgentPolicy", lockedAt: STAMP as never },
      issue: {
        code: "SECRET_LOCKED",
        retryable: true,
        action: "unlockFyAgent",
        lockSource: "fyAgentPolicy",
      },
    }),
    refAggregate(REF.backendLocked, DISPLAY.backendLocked, "locked", {
      lock: { source: "backend", lockedAt: STAMP as never },
      issue: {
        code: "SECRET_LOCKED",
        retryable: true,
        action: "unlockBackend",
        lockSource: "backend",
      },
    }),
    refAggregate(REF.missing, DISPLAY.missing, "missing", {
      issue: {
        code: "SECRET_MISSING",
        retryable: true,
        action: "captureReplacement",
      },
    }),
    refAggregate(REF.revoked, DISPLAY.revoked, "revoked", {
      revocation: { source: "userDelete", revokedAt: STAMP as never },
      issue: {
        code: "SECRET_REVOKED",
        retryable: false,
        action: "none",
        revocationSource: "userDelete",
      },
    }),
    refAggregate(REF.unavailable, DISPLAY.unavailable, "unavailable", {
      issue: {
        code: "SECRET_BACKEND_UNAVAILABLE",
        retryable: false,
        action: "reconnectDevice",
        backendUnavailableReason: "hardwareUnregistered",
      },
    }),
    refAggregate(REF.candidate, DISPLAY.candidate, "ready"),
    refAggregate(REF.discard, DISPLAY.discard, "ready"),
    refAggregate(REF.expired, DISPLAY.expired, "ready"),
    refAggregate(REF.cleanup, DISPLAY.cleanup, "stale", {
      issue: {
        code: "SECRET_OPERATION_RECOVERY_REQUIRED",
        retryable: true,
        action: "completeRecovery",
      },
    }),
    refAggregate(REF.denied, DISPLAY.denied, "denied", {
      issue: {
        code: "SECRET_PERMISSION_DENIED",
        retryable: true,
        action: "requestPermission",
      },
    }),
    refAggregate(REF.stale, DISPLAY.stale, "stale", {
      issue: {
        code: "SECRET_STALE",
        retryable: true,
        action: "completeRecovery",
      },
    }),
  ],
  candidates: [
    {
      schemaVersion: 1,
      candidateId: hexId("scd", "1", "1001") as never,
      candidateRevision: 1 as never,
      kind: "newBinding",
      comparisonPolicy: "candidateEquality",
      comparisonImpact: {
        policy: "candidateEquality",
        userMeaning: "verifySameValueMigration",
      },
      state: "verifiedPendingPlan",
      secretRefDisplay: DISPLAY.candidate,
      purpose: "codexApiKey",
      targetOwners: [owner(OWNERS.candidate)],
      legacySourceCounts: [{ category: "providerAuthJson", count: 1 }],
      createdAt: STAMP as never,
      expiresAt: EXPIRES as never,
    },
    {
      schemaVersion: 1,
      candidateId: hexId("scd", "2", "2002") as never,
      candidateRevision: 1 as never,
      kind: "replaceBinding",
      comparisonPolicy: "explicitReplacement",
      comparisonImpact: {
        policy: "explicitReplacement",
        userMeaning: "replaceExistingCredential",
        affectedSourceCount: 2,
        replacesBoundBinding: true,
      },
      state: "verifiedPendingPlan",
      secretRefDisplay: DISPLAY.discard,
      purpose: "codexApiKey",
      targetOwners: [owner(OWNERS.discard)],
      legacySourceCounts: [
        { category: "providerAuthJson", count: 1 },
        { category: "providerConfigTomlTopLevel", count: 1 },
      ],
      createdAt: STAMP as never,
      expiresAt: EXPIRES as never,
      pendingTerminalDisposition: "discarded",
      issue: {
        code: "SECRET_OPERATION_RECOVERY_REQUIRED",
        retryable: true,
        action: "discardCandidate",
      },
    },
    {
      schemaVersion: 1,
      candidateId: hexId("scd", "3", "3003") as never,
      candidateRevision: 1 as never,
      kind: "legacyReconcile",
      comparisonPolicy: "explicitReplacement",
      comparisonImpact: {
        policy: "explicitReplacement",
        userMeaning: "replaceExistingCredential",
        affectedSourceCount: 1,
        replacesBoundBinding: false,
      },
      state: "expired",
      secretRefDisplay: DISPLAY.expired,
      purpose: "codexApiKey",
      targetOwners: [owner(OWNERS.expired)],
      legacySourceCounts: [{ category: "providerAuthJson", count: 1 }],
      createdAt: STAMP as never,
      expiresAt: EXPIRES as never,
    },
    {
      schemaVersion: 1,
      candidateId: hexId("scd", "4", "4004") as never,
      candidateRevision: 1 as never,
      kind: "rotateBindingSet",
      comparisonPolicy: "candidateEquality",
      comparisonImpact: {
        policy: "candidateEquality",
        userMeaning: "verifySameValueMigration",
      },
      state: "cleanupRequired",
      secretRefDisplay: DISPLAY.cleanup,
      purpose: "codexApiKey",
      targetOwners: [owner(OWNERS.cleanup)],
      legacySourceCounts: [],
      createdAt: STAMP as never,
      expiresAt: EXPIRES as never,
      issue: {
        code: "SECRET_OPERATION_RECOVERY_REQUIRED",
        retryable: true,
        action: "completeRecovery",
      },
    },
  ],
  captureIntent: {
    schemaVersion: 1,
    captureIntentId: hexId("sci", "d", "5005") as never,
    owner: owner(OWNERS.unbound),
    purpose: "codexApiKey",
    intent: "newBinding",
    currentBinding: { state: "unbound" },
    legacySourceCoverage: clearCoverage,
    expiresAt: EXPIRES as never,
  },
  registeredBackends: [{ backend: osKeyring }],
  secretDeleteImpact: {
    impact: {
      schemaVersion: 1,
      secretRefDisplay: DISPLAY.ready,
      bindingSetCas: {
        revision: 1 as never,
        digest: DIGEST as never,
        count: 2,
      },
      affectedOwners: [
        {
          owner: owner(OWNERS.ready),
          purpose: "codexApiKey",
          bindingRevision: 1 as never,
          createdAt: STAMP as never,
          updatedAt: STAMP as never,
        },
        {
          owner: owner(OWNERS.shared),
          purpose: "codexApiKey",
          bindingRevision: 1 as never,
          createdAt: STAMP as never,
          updatedAt: STAMP as never,
        },
      ],
      effect: "allBindingsAffected",
      noFallback: true,
    },
    readiness: { status: "ready" },
  },
  providerDeleteReady: {
    schemaVersion: 1,
    status: "ready",
    impact: {
      bindingState: "bound",
      providerDeleteImpactId: hexId("pdi", "e", "6006") as never,
      owner: owner(OWNERS.ready),
      existingBinding: {
        state: "bound",
        secretRefDisplay: DISPLAY.ready,
        remainingOwners: [owner(OWNERS.shared)],
        becomesOrphan: true,
      },
      legacySourceCoverage: clearCoverage,
      deleteAllowed: true,
      effect: "none",
      secretRetained: true,
      separateSecretDeleteAction: "get_secret_delete_impact",
    },
  },
  providerDeleteBlocked: {
    schemaVersion: 1,
    status: "blockedLegacyResolutionRequired",
    blocked: {
      bindingState: "legacy",
      owner: owner(OWNERS.legacy),
      existingBinding: {
        state: "bound",
        secretRefDisplay: DISPLAY.ready,
        remainingOwners: [owner(OWNERS.shared)],
        becomesOrphan: false,
      },
      legacySourceCoverage: blockingCoverage,
      deleteAllowed: false,
      effect: "none",
      action: "resolveLegacyConflict",
    },
  },
  hardwareConfirmation: {
    operation: "delete",
    device: {
      displayName: "演示安全密钥" as never,
      deviceClass: "securityKey",
      transport: "usb",
    },
    timeoutSeconds: 30 as never,
    promptKey: "secret.hardware.confirmTouch",
  },
};

export const credentialOwnerIds = OWNERS;

export function createBrowserCredentialsPort(): CredentialsPort {
  return {
    source: "browser",
    async listWorkspace() {
      return decodeCredentialsSnapshot(
        JSON.parse(JSON.stringify(credentialBrowserFixtures)),
      );
    },
    async beginCapture(request: BeginSecretCaptureRequest) {
      decodeBeginSecretCaptureRequest(request);
    },
  };
}
