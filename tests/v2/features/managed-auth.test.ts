import { describe, expect, it } from "vitest";

import {
  assertManagedAuthConnectionActionRequest,
  assertStartManagedAuthLoginRequest,
  parseManagedAuthCommandError,
  parseManagedAuthLoginSession,
  parseManagedAuthMutationResult,
  parseManagedAuthOverview,
  parseManagedAuthRemovalPreview,
} from "@/v2/shared/features/managed-auth";
import {
  ACCOUNT_REVISION,
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
  mutationResultFixture,
  removalPreviewFixture,
} from "../fixtures/managedAuth";

describe("managed auth wire contract", () => {
  it("keeps account identity, software connection, and request source separate", () => {
    const overview = parseManagedAuthOverview(managedAuthOverviewFixture());
    const account = overview.accounts.find(
      (item) => item.accountId === OPENAI_ACCOUNT_ID,
    );
    const codex = overview.connections.find(
      (item) => item.connectionId === CODEX_CONNECTION_ID,
    );

    expect(account).toMatchObject({
      login: "person@example.com",
      provider: "openai",
      connectedConsumerCount: 2,
    });
    expect(codex).toMatchObject({
      accountId: OPENAI_ACCOUNT_ID,
      requestMode: "third_party_api",
      requestProviderLabel: "DeepSeek API",
      officialSessionPreserved: true,
    });
  });

  it("rejects excess secret-shaped fields and dangling account references", () => {
    const withToken = structuredClone(
      managedAuthOverviewFixture(),
    ) as unknown as Record<string, unknown>;
    const accounts = withToken.accounts as Array<Record<string, unknown>>;
    accounts[0] = { ...accounts[0], refreshToken: "sentinel" };
    expect(() => parseManagedAuthOverview(withToken)).toThrow(
      "账号与认证数据不可用",
    );

    const dangling = structuredClone(managedAuthOverviewFixture());
    dangling.connections[0].accountId = `ma1:${"9".repeat(32)}`;
    expect(() => parseManagedAuthOverview(dangling)).toThrow(
      "账号与认证数据不可用",
    );
  });

  it("cross-checks connected consumer counts instead of trusting summaries", () => {
    const overview = structuredClone(managedAuthOverviewFixture());
    overview.accounts[0].connectedConsumerCount = 1;
    expect(() => parseManagedAuthOverview(overview)).toThrow(
      "账号与认证数据不可用",
    );
  });

  it("accepts device-code sessions but never exposes browser callback material", () => {
    expect(
      parseManagedAuthLoginSession(deviceLoginSessionFixture()),
    ).toMatchObject({
      method: "device_code",
      userCode: "ABCD-EFGH",
      verificationUri: "https://auth.openai.com/codex/device",
    });

    expect(() =>
      parseManagedAuthLoginSession({
        ...deviceLoginSessionFixture({
          method: "browser_loopback",
          canSwitchToDeviceCode: true,
        }),
        authorizationUrl: "https://auth.openai.com/oauth/authorize?code=x",
      }),
    ).toThrow("账号与认证数据不可用");
  });

  it("parses removal previews and authoritative mutation results", () => {
    expect(
      parseManagedAuthRemovalPreview(removalPreviewFixture()),
    ).toMatchObject({
      accountId: OPENAI_ACCOUNT_ID,
      canApply: true,
    });
    expect(
      parseManagedAuthMutationResult(mutationResultFixture()),
    ).toMatchObject({
      outcome: "completed",
      reasonCode: null,
    });
  });

  it("validates closed mutation requests before native IPC", () => {
    expect(
      assertStartManagedAuthLoginRequest({
        provider: "openai",
        purpose: "connect_consumer",
        consumer: "codex",
        method: "browser_loopback",
        accountId: null,
      }),
    ).toEqual({
      provider: "openai",
      purpose: "connect_consumer",
      consumer: "codex",
      method: "browser_loopback",
      accountId: null,
    });

    expect(() =>
      assertStartManagedAuthLoginRequest({
        provider: "xai",
        purpose: "connect_consumer",
        consumer: "grokbuild",
        method: "browser_loopback",
        accountId: null,
      }),
    ).toThrow("账号与认证请求无效");

    expect(
      assertManagedAuthConnectionActionRequest({
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: CONNECTION_REVISION,
        action: "switch_account",
        accountId: OPENAI_ACCOUNT_ID,
      }),
    ).toMatchObject({ action: "switch_account" });

    expect(() =>
      assertManagedAuthConnectionActionRequest({
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: ACCOUNT_REVISION,
        action: "disconnect",
        accountId: OPENAI_ACCOUNT_ID,
      }),
    ).toThrow("账号与认证请求无效");
  });

  it("parses managed-auth command errors without treating them as generic failures", () => {
    expect(
      parseManagedAuthCommandError({
        contractVersion: 1,
        reasonCode: "secret_unavailable",
      }),
    ).toBe("secret_unavailable");
    expect(
      parseManagedAuthCommandError({
        contractVersion: 1,
        reasonCode: "external_change_detected",
      }),
    ).toBe("external_change_detected");
    expect(parseManagedAuthCommandError({ reasonCode: "secret_unavailable" })).toBe(
      null,
    );
  });
});
