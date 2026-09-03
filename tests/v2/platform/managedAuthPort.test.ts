import { beforeEach, describe, expect, it, vi } from "vitest";

import { createManagedAuthPort } from "@/v2/shared/platform/tauri/feature-ports/managedAuth";
import {
  ACCOUNT_REVISION,
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  PREVIEW_ID,
  SESSION_ID,
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
  mutationResultFixture,
  removalPreviewFixture,
} from "../fixtures/managedAuth";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Tauri managed auth port", () => {
  beforeEach(() => invoke.mockReset());

  it("uses only closed commands and bounded payloads", async () => {
    const port = createManagedAuthPort();

    invoke.mockResolvedValueOnce(managedAuthOverviewFixture());
    await port.getOverview();
    expect(invoke).toHaveBeenLastCalledWith("managed_auth_get_overview");

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
    await port.applyConnectionAction({
      connectionId: CODEX_CONNECTION_ID,
      expectedRevision: CONNECTION_REVISION,
      action: "disconnect",
      accountId: null,
    });
    expect(invoke).toHaveBeenLastCalledWith(
      "managed_auth_apply_connection_action",
      {
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
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValueOnce({
      ...managedAuthOverviewFixture(),
      token: "sentinel",
    });
    await expect(port.getOverview()).rejects.toThrow("账号与认证数据不可用");
  });
});
