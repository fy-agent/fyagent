import { describe, expect, it } from "vitest";

import {
  PROVIDER_LABELS,
  SUPPORTED_PROVIDER_IDS,
  canExportSession,
  localProviderProbeSchema,
  migratableSessionSchema,
  restoreAttemptSchema,
  sessionPackageSchema,
} from "@/shared/features/session-migration";
import {
  disabledProbeSample,
  exactFinalText,
  exactUserText,
  restoreAttemptSample,
  sessionPackageSample,
} from "./samples";

function clone<T>(value: T): T {
  return structuredClone(value);
}

function packageParts(value: Record<string, unknown>) {
  const exporter = value.exporter as Record<string, unknown>;
  const session = (value.sessions as Array<Record<string, unknown>>)[0];
  const origin = session.origin as Record<string, unknown>;
  const message = (session.messages as Array<Record<string, unknown>>)[0];
  const extraction = session.extraction as Record<string, unknown>;
  const omitted = extraction.omitted as Record<string, unknown>;
  return { exporter, session, origin, message, extraction, omitted };
}

describe("session migration strict schemas", () => {
  it("uses the backend provider identifier for Grok Build", () => {
    expect(SUPPORTED_PROVIDER_IDS).toContain("grokbuild");
    expect(SUPPORTED_PROVIDER_IDS).not.toContain("grok");
    expect(PROVIDER_LABELS.grokbuild).toBe("Grok");
  });

  it("preserves body code points while parsing the closed package", () => {
    const parsed = sessionPackageSchema.parse(sessionPackageSample());
    const [user, assistant] = parsed.sessions[0].messages;

    expect(user.text).toBe(exactUserText);
    expect(assistant.text).toBe(exactFinalText);
    expect(user.text).toContain("\r\n\r\n");
    expect(user.text).toContain("/Users/alice/demo");
    expect(user.text).toContain("D:\\work\\demo");
  });

  it.each<[string, (value: Record<string, unknown>) => void]>([
    ["envelope", (value) => void (value.toolCalls = [])],
    [
      "exporter",
      (value) => void (packageParts(value).exporter.token = "SECRET"),
    ],
    [
      "session",
      (value) => void (packageParts(value).session.sourcePath = "/private"),
    ],
    ["origin", (value) => void (packageParts(value).origin.command = "rm -rf")],
    ["message", (value) => void (packageParts(value).message.reasoning = {})],
    [
      "extraction",
      (value) => void (packageParts(value).extraction.rawEvents = []),
    ],
    [
      "omitted counts",
      (value) =>
        void (packageParts(value).omitted.toolOutput = "LEAK_TOOL_OUT_92"),
    ],
  ])("rejects an unknown field at the %s level", (_level, mutate) => {
    const candidate = clone(sessionPackageSample());
    mutate(candidate);
    expect(() => sessionPackageSchema.parse(candidate)).toThrow();
  });

  it("rejects unsupported package schema and unknown message kinds", () => {
    const unsupported = clone(sessionPackageSample());
    unsupported.schema = "fyagent.session.v2";
    expect(() => sessionPackageSchema.parse(unsupported)).toThrow();

    const commentary = clone(sessionPackageSample());
    const sessions = commentary.sessions as Array<Record<string, unknown>>;
    const messages = sessions[0].messages as Array<Record<string, unknown>>;
    messages[1].kind = "assistantCommentary";
    expect(() => sessionPackageSchema.parse(commentary)).toThrow();
  });

  it("keeps a genuinely unanswered user message exportable without inventing an assistant", () => {
    const candidate = clone(sessionPackageSample());
    const session = (candidate.sessions as Array<Record<string, unknown>>)[0];
    const messages = session.messages as Array<Record<string, unknown>>;
    messages.push({
      seq: 2,
      kind: "userText",
      text: "C：尾部未完成输入。",
    });
    const extraction = session.extraction as Record<string, unknown>;
    extraction.openUserMessages = [2];

    const parsed = migratableSessionSchema.parse(session);
    expect(canExportSession(parsed)).toEqual({ allowed: true });
    expect(parsed.messages).toHaveLength(3);
    expect(parsed.messages.at(-1)).toEqual({
      seq: 2,
      kind: "userText",
      text: "C：尾部未完成输入。",
    });
  });

  it("does not let user attestation alter the system stage", () => {
    const parsed = restoreAttemptSchema.parse(
      restoreAttemptSample({
        stage: "nativeWritten",
        userAttestation: {
          attestedAt: 1_795_478_402_000,
          claimedStage: "nextTurnReplyVerified",
          note: "用户自报，不是系统证据",
        },
      }),
    );

    expect(parsed.stage).toBe("nativeWritten");
    expect(parsed.userAttestation?.claimedStage).toBe("nextTurnReplyVerified");
  });

  it("keeps disabled runtime capability false as an independent fact", () => {
    const probe = localProviderProbeSchema.parse(disabledProbeSample());
    const userAttestation = {
      claimedStage: "nextTurnReplyVerified",
      workspace: "/tmp/changed",
    };

    expect(userAttestation.claimedStage).toBe("nextTurnReplyVerified");
    expect(probe.writeSupported).toBe(false);
    expect(probe.reasonCode).toBe("providerVersionUnsupported");
  });
});
