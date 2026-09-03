import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  LEGACY_AUTH_MUTATION_DISABLED,
  authCancelLogin,
  authGetStatus,
  authListAccounts,
  authLogout,
  authPollForAccount,
  authRemoveAccount,
  authSetDefaultAccount,
  authStartLogin,
} from "@/lib/api/auth";
import {
  copilotGetToken,
  copilotGetTokenForAccount,
  copilotLogout,
  copilotPollForAccount,
  copilotPollForAuth,
  copilotRemoveAccount,
  copilotSetDefaultAccount,
  copilotStartDeviceFlow,
} from "@/lib/api/copilot";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("leftover auth API", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("does not invoke leftover login, poll, remove, default, logout, or cancel", async () => {
    await expect(authStartLogin("codex_oauth")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(
      authPollForAccount("codex_oauth", "device-code"),
    ).rejects.toThrow(LEGACY_AUTH_MUTATION_DISABLED);
    await expect(authRemoveAccount("codex_oauth", "account-1")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(
      authSetDefaultAccount("codex_oauth", "account-1"),
    ).rejects.toThrow(LEGACY_AUTH_MUTATION_DISABLED);
    await expect(authLogout("codex_oauth")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(authCancelLogin("codex_oauth", "device-code")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("does not invoke leftover Copilot login, poll, remove, default, or logout", async () => {
    await expect(copilotStartDeviceFlow()).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(copilotPollForAuth("device-code")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(copilotPollForAccount("device-code")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(copilotRemoveAccount("account-1")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(copilotSetDefaultAccount("account-1")).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    await expect(copilotLogout()).rejects.toThrow(
      LEGACY_AUTH_MUTATION_DISABLED,
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("does not invoke leftover Copilot token IPC", async () => {
    await expect(copilotGetToken()).rejects.toThrow(
      "copilot_token_not_exposed",
    );
    await expect(copilotGetTokenForAccount("account-1")).rejects.toThrow(
      "copilot_token_not_exposed",
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("still reads leftover account status through list and get-status", async () => {
    invokeMock.mockResolvedValueOnce([]);
    invokeMock.mockResolvedValueOnce({
      provider: "codex_oauth",
      authenticated: false,
      default_account_id: null,
      accounts: [],
    });

    await expect(authListAccounts("codex_oauth")).resolves.toEqual([]);
    await expect(authGetStatus("codex_oauth")).resolves.toMatchObject({
      provider: "codex_oauth",
      authenticated: false,
    });
    expect(invokeMock).toHaveBeenNthCalledWith(1, "auth_list_accounts", {
      authProvider: "codex_oauth",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "auth_get_status", {
      authProvider: "codex_oauth",
    });
  });
});
