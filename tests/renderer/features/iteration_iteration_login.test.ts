import { describe, expect, it } from "vitest";

import {
  parseManagedAuthLoginSession,
  parseManagedAuthOverview,
  type ManagedAuthLoginStage,
} from "@/shared/features/managed-auth";
import {
  OPENAI_ACCOUNT_ID,
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
} from "../fixtures/managedAuth";

const emptyDeviceFields = {
  userCode: null,
  verificationUri: null,
  expiresAt: null,
};

describe("I03/F29 device login stage boundary", () => {
  it.each(["openai", "xai"] as const)(
    "accepts the %s native Preparing response before the device grant arrives",
    (provider) => {
      const response = deviceLoginSessionFixture({
        provider,
        officialHost: provider === "openai" ? "auth.openai.com" : "auth.x.ai",
        purpose: "save_only",
        consumer: null,
        stage: "preparing",
        ...emptyDeviceFields,
      });

      expect(parseManagedAuthLoginSession(response)).toEqual(response);
    },
  );

  it("recovers a Preparing session from the actual overview parser", () => {
    const response = managedAuthOverviewFixture();
    const session = deviceLoginSessionFixture({
      stage: "preparing",
      ...emptyDeviceFields,
    });
    response.activeSessions = [session];

    expect(parseManagedAuthOverview(response).activeSessions).toEqual([
      session,
    ]);
  });

  it("accepts AwaitingUser with all device fields and the browser Preparing control", () => {
    expect(
      parseManagedAuthLoginSession(deviceLoginSessionFixture()),
    ).toMatchObject({
      stage: "awaiting_user",
      userCode: "ABCD-EFGH",
      verificationUri: "https://auth.openai.com/codex/device",
      expiresAt: "2026-09-03T08:15:00Z",
    });
    expect(
      parseManagedAuthLoginSession(
        deviceLoginSessionFixture({
          method: "browser_loopback",
          stage: "preparing",
          canSwitchToDeviceCode: true,
          ...emptyDeviceFields,
        }),
      ),
    ).toMatchObject({ method: "browser_loopback", stage: "preparing" });
  });

  const grantStages: ManagedAuthLoginStage[] = [
    "opening_browser",
    "awaiting_user",
    "exchanging_code",
    "saving_account",
    "connecting_consumer",
    "verifying",
  ];
  const requiredFields = ["userCode", "verificationUri", "expiresAt"] as const;
  const incompleteGrants = grantStages.flatMap((stage) =>
    requiredFields.map((field) => ({ stage, field })),
  );
  it.each(incompleteGrants)(
    "rejects $stage when $field is absent after Preparing",
    ({ stage, field }) => {
      expect(() =>
        parseManagedAuthLoginSession({
          ...deviceLoginSessionFixture({ stage }),
          [field]: null,
        }),
      ).toThrow("账号与认证数据不可用");
    },
  );

  it.each([
    { stage: "completed", reasonCode: null, accountId: OPENAI_ACCOUNT_ID },
    {
      stage: "partial",
      reasonCode: "partial_completion",
      accountId: OPENAI_ACCOUNT_ID,
    },
    { stage: "failed", reasonCode: "login_failed", accountId: null },
    { stage: "cancelled", reasonCode: "cancelled", accountId: null },
    { stage: "expired", reasonCode: "device_code_expired", accountId: null },
  ] as const)(
    "retains the nullable device fields on terminal $stage",
    ({ stage, reasonCode, accountId }) => {
      const response = deviceLoginSessionFixture({
        stage,
        reasonCode,
        accountId,
        terminal: true,
        canCancel: false,
        canRetry: stage === "failed" || stage === "expired",
        ...emptyDeviceFields,
      });
      expect(parseManagedAuthLoginSession(response)).toEqual(response);
    },
  );

  it.each([
    { officialHost: "example.com" },
    { verificationUri: "https://example.com/device" },
    { verificationUri: "http://auth.openai.com/codex/device" },
    { verificationUri: "https://auth.openai.com/codex/device?code=sentinel" },
    { verificationUri: "https://auth.openai.com/codex/device#sentinel" },
    { verificationUri: "https://sentinel@auth.openai.com/codex/device" },
    { userCode: "secret sentinel" },
    { expiresAt: "tomorrow" },
    { sessionId: "not-a-session" },
    { stage: "unknown" },
    { contractVersion: 2 },
    { deviceCode: "private-sentinel" },
    { terminal: true },
    { canCancel: false },
    { canRetry: true },
  ])("keeps Preparing strict for malformed input %j", (invalidFields) => {
    expect(() =>
      parseManagedAuthLoginSession({
        ...deviceLoginSessionFixture({
          stage: "preparing",
          ...emptyDeviceFields,
        }),
        ...invalidFields,
      }),
    ).toThrow("账号与认证数据不可用");
  });
});
