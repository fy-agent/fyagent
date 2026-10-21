import { beforeEach, describe, expect, it, vi } from "vitest";

import { createManagedAuthPort } from "@/shared/platform/tauri/feature-ports/managedAuth";
import {
  ACCOUNT_REVISION,
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  PREVIEW_ID,
  SESSION_ID,
  accountQuotaFixture,
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
  mutationResultFixture,
  removalPreviewFixture,
} from "../fixtures/managedAuth";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Tauri managed auth port", () => {
  beforeEach(() => invoke.mockReset());

  it("reads one account's OpenAI catalog and rejects any other shape", async () => {
    const port = createManagedAuthPort();
    const catalog = {
      contractVersion: 1,
      accountId: OPENAI_ACCOUNT_ID,
      fetchedAt: "2026-10-22T04:30:00Z",
      models: ["gpt-6-sol", "gpt-5.5"],
    };
    invoke.mockResolvedValueOnce(catalog);
    await expect(port.getOpenAiModels(OPENAI_ACCOUNT_ID)).resolves.toEqual(
      catalog,
    );
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_get_openai_models", {
      accountId: OPENAI_ACCOUNT_ID,
    });

    for (const invalid of [
      { ...catalog, accountId: `ma1:${"9".repeat(32)}` },
      { ...catalog, token: "leak" },
      { ...catalog, models: ["bad id"] },
      { ...catalog, models: ["gpt-5.5", "gpt-5.5"] },
      { ...catalog, models: Array.from({ length: 501 }, (_, i) => `m-${i}`) },
      { ...catalog, fetchedAt: "yesterday" },
      { ...catalog, contractVersion: 2 },
    ]) {
      invoke.mockResolvedValueOnce(invalid);
      await expect(port.getOpenAiModels(OPENAI_ACCOUNT_ID)).rejects.toThrow();
    }
    invoke.mockClear();
    await expect(port.getOpenAiModels("not-an-account")).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("uses only closed commands and bounded payloads", async () => {
    const port = createManagedAuthPort();

    invoke.mockResolvedValueOnce(managedAuthOverviewFixture());
    await port.getOverview();
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_get_overview");

    invoke.mockResolvedValueOnce(accountQuotaFixture(OPENAI_ACCOUNT_ID));
    await port.getAccountQuota(OPENAI_ACCOUNT_ID);
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_get_account_quota", {
      accountId: OPENAI_ACCOUNT_ID,
    });

    invoke.mockResolvedValueOnce(deviceLoginSessionFixture());
    await port.startLogin({
      provider: "openai",
      purpose: "connect_consumer",
      consumer: "codex",
      method: "device_code",
      accountId: null,
    });
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_start_login", {
      request: {
        provider: "openai",
        purpose: "connect_consumer",
        consumer: "codex",
        method: "device_code",
        accountId: null,
      },
    });

    invoke.mockResolvedValueOnce(deviceLoginSessionFixture());
    await port.getLoginSession(SESSION_ID);
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_get_login_session", {
      sessionId: SESSION_ID,
    });

    invoke.mockResolvedValueOnce(
      deviceLoginSessionFixture({
        stage: "cancelled",
        canCancel: false,
        reasonCode: "cancelled",
        terminal: true,
      }),
    );
    await port.cancelLogin(SESSION_ID);
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_cancel_login", {
      sessionId: SESSION_ID,
    });

    invoke.mockResolvedValueOnce(mutationResultFixture());
    await port.setDefaultAccount(OPENAI_ACCOUNT_ID, ACCOUNT_REVISION);
    expect(invoke).toHaveBeenLastCalledWith(
      "managed_auth_set_default_account",
      {
        request: {
          accountId: OPENAI_ACCOUNT_ID,
          expectedRevision: ACCOUNT_REVISION,
        },
      },
    );

    invoke.mockResolvedValueOnce(removalPreviewFixture());
    await port.previewAccountRemoval(OPENAI_ACCOUNT_ID, ACCOUNT_REVISION);
    expect(invoke).toHaveBeenLastCalledWith(
      "managed_auth_preview_account_removal",
      {
        request: {
          accountId: OPENAI_ACCOUNT_ID,
          expectedRevision: ACCOUNT_REVISION,
        },
      },
    );

    invoke.mockResolvedValueOnce(mutationResultFixture());
    await port.removeAccount(PREVIEW_ID, OPENAI_ACCOUNT_ID, ACCOUNT_REVISION);
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_remove_account", {
      request: {
        previewId: PREVIEW_ID,
        accountId: OPENAI_ACCOUNT_ID,
        expectedRevision: ACCOUNT_REVISION,
      },
    });

    invoke.mockResolvedValueOnce(mutationResultFixture());
    await port.applyConnectionAction(
      {
        connectionId: CODEX_CONNECTION_ID,
        expectedRevision: CONNECTION_REVISION,
        action: "disconnect",
        accountId: null,
      },
      "323e4567-e89b-42d3-a456-426614174000",
    );
    expect(invoke).toHaveBeenLastCalledWith(
      "managed_auth_apply_connection_action",
      {
        previewId: "323e4567-e89b-42d3-a456-426614174000",
        request: {
          connectionId: CODEX_CONNECTION_ID,
          expectedRevision: CONNECTION_REVISION,
          action: "disconnect",
          accountId: null,
        },
      },
    );
  });

  it("rejects malformed requests before IPC and excess response fields", async () => {
    const port = createManagedAuthPort();
    await expect(port.getLoginSession("not-a-session")).rejects.toThrow(
      "账号与认证请求无效",
    );
    await expect(port.getAccountQuota("not-an-account")).rejects.toThrow(
      "账号与认证请求无效",
    );
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValueOnce({
      ...managedAuthOverviewFixture(),
      token: "sentinel",
    });
    await expect(port.getOverview()).rejects.toThrow("账号与认证数据不可用");
  });
});
