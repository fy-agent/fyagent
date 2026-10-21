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

  it("reads the native Codex catalog only through the attributed contract", async () => {
    const port = createManagedAuthPort();
    const ready = {
      contractVersion: 1,
      accountId: OPENAI_ACCOUNT_ID,
      source: "codex_app_server",
      accountBinding: "matched",
      bindingDetail: null,
      nativeAccountId: OPENAI_ACCOUNT_ID,
      providerRoute: "official",
      revision: CONNECTION_REVISION,
      checkedAt: "2026-10-22T04:30:00Z",
      catalogStatus: "ready",
      failure: null,
      cliVersion: "0.160.1",
      truncated: false,
      models: [
        {
          id: "gpt-6-sol",
          displayName: "GPT-6 Sol",
          isDefault: true,
          reasoningEfforts: ["low", "high"],
        },
        {
          id: "gpt-5.5",
          displayName: null,
          isDefault: false,
          reasoningEfforts: [],
        },
      ],
    };
    invoke.mockResolvedValueOnce(ready);
    await expect(port.getCodexNativeModels(OPENAI_ACCOUNT_ID)).resolves.toEqual(
      ready,
    );
    expect(invoke).toHaveBeenLastCalledWith(
      "managed_auth_get_codex_native_models",
      { accountId: OPENAI_ACCOUNT_ID },
    );

    const notRead = {
      ...ready,
      accountBinding: "unknown",
      bindingDetail: "signed_out",
      nativeAccountId: null,
      catalogStatus: "not_read",
      cliVersion: null,
      models: [],
    };
    invoke.mockResolvedValueOnce(notRead);
    await expect(
      port.getCodexNativeModels(OPENAI_ACCOUNT_ID),
    ).resolves.toMatchObject({ catalogStatus: "not_read" });

    invoke.mockResolvedValueOnce({
      ...notRead,
      bindingDetail: "unsupported_credential_store",
    });
    await expect(
      port.getCodexNativeModels(OPENAI_ACCOUNT_ID),
    ).resolves.toMatchObject({
      accountBinding: "unknown",
      bindingDetail: "unsupported_credential_store",
      catalogStatus: "not_read",
    });

    const model = ready.models[0];
    for (const invalid of [
      { ...ready, accountId: `ma1:${"9".repeat(32)}` },
      { ...ready, token: "leak" },
      { ...ready, source: "chatgpt_backend" },
      { ...ready, revision: "r1" },
      { ...ready, providerRoute: "deepseek" },
      { ...ready, providerRoute: null },
      // Models only exist for a matched login.
      { ...ready, accountBinding: "mismatch" },
      { ...notRead, models: ready.models },
      { ...notRead, bindingDetail: null },
      { ...ready, bindingDetail: "signed_out" },
      { ...ready, catalogStatus: "failed" },
      { ...ready, failure: "timed_out" },
      { ...ready, cliVersion: "v1;rm" },
      { ...ready, models: [model, model] },
      { ...ready, models: [model, { ...model, id: "other" }] },
      { ...ready, models: [{ ...model, id: "has space" }] },
      { ...ready, models: [{ ...model, secret: "x" }] },
      { ...ready, models: [{ ...model, reasoningEfforts: ["LOUD"] }] },
      {
        ...ready,
        models: Array.from({ length: 65 }, (_, i) => ({
          ...model,
          id: `m-${i}`,
          isDefault: false,
        })),
      },
    ]) {
      invoke.mockResolvedValueOnce(invalid);
      await expect(
        port.getCodexNativeModels(OPENAI_ACCOUNT_ID),
      ).rejects.toThrow();
    }
    invoke.mockClear();
    await expect(port.getCodexNativeModels("not-an-account")).rejects.toThrow();
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
