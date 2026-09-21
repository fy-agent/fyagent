const digest = `fyc1:${"a".repeat(64)}`;
const snapshotId = `fys1:${"b".repeat(64)}`;

export const exactUserText =
  '保留这一段 🪁、CRLF：\r\n\r\n- macOS：`/Users/alice/demo`\r\n- Windows：`D:\\work\\demo`\r\n\r\n```ts\r\nconst answer = "FINAL-T2-KEEP";\r\n```';

export const exactFinalText =
  '路径 `/Users/alice/demo` 与 `D:\\work\\demo` 保持原文。\r\n```ts\r\nconst answer = "FINAL-T2-KEEP";\r\n```';

export function sessionPackageSample(): Record<string, unknown> {
  return {
    schema: "fyagent.session.v1",
    exportedAt: 1_795_478_400_000,
    exporter: {
      app: "fyagent",
      appVersion: "0.4.6",
      platform: "macos",
    },
    sessions: [
      {
        snapshotId,
        contentDigest: digest,
        origin: {
          originId: "11111111-1111-4111-8111-111111111111",
          providerId: "codex",
          sessionId: "synthetic-session",
          cliVersion: "0.154.0",
          storeFingerprint: "synthetic-store",
        },
        title: "合成迁移会话",
        workspaceLabel: "demo",
        messages: [
          { seq: 0, kind: "userText", text: exactUserText },
          { seq: 1, kind: "assistantFinal", text: exactFinalText },
        ],
        extraction: {
          ruleId: "codex.rollout.phase-v1",
          ruleVerifiedVersions: ["=0.154.0"],
          openUserMessages: [],
          omitted: {
            toolEvents: 2,
            reasoningBlocks: 1,
            commentaryMessages: 1,
            attachments: 1,
            runtimeInjections: 2,
            unknownBlocks: 0,
          },
          sourcePathFamily: "posix",
        },
      },
    ],
  };
}

export function restoreAttemptSample(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    attemptId: "22222222-2222-4222-8222-222222222222",
    requestId: "33333333-3333-4333-8333-333333333333",
    snapshotId,
    requestKind: "defaultImport",
    idempotencySlot: `fyi1:${"c".repeat(64)}`,
    contentDigest: digest,
    origin: {
      originId: "11111111-1111-4111-8111-111111111111",
      providerId: "codex",
      sessionId: "synthetic-session",
    },
    targetProviderId: "codex",
    targetStoreId: "synthetic-target-store",
    targetNativeNonce: "synthetic-native-nonce",
    targetNativeId: "synthetic-native-id",
    targetWorkspace: "/tmp/fyagent-session-migration",
    stage: "nativeWritten",
    attemptCount: 1,
    userAttestation: null,
    createdAt: 1_795_478_400_000,
    updatedAt: 1_795_478_401_000,
    ...overrides,
  };
}

export function disabledProbeSample(): Record<string, unknown> {
  return {
    providerId: "grokbuild",
    installed: true,
    detectedVersion: "1.0.34",
    extractionSupported: false,
    writeSupported: false,
    reasonCode: "providerVersionUnsupported",
  };
}
