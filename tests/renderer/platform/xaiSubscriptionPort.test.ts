import { beforeEach, describe, expect, it, vi } from "vitest";

import type { BindXaiManagedRequest } from "@/shared/features/models";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/shared/platform/browser/features";
import { XAI_ACCOUNT_ID } from "../fixtures/managedAuth";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const request: BindXaiManagedRequest = {
  app: "claude",
  accountId: XAI_ACCOUNT_ID,
  modelId: "grok-fixture-1",
};
const result = {
  providerId: "xai-managed-claude-account-1",
  providerName: "SuperGrok fixture",
  app: "claude",
  alreadyBound: false,
  activated: true,
};

async function ports() {
  const { createModelFeaturePorts } = await import(
    "@/shared/platform/tauri/feature-ports/models"
  );
  return createModelFeaturePorts().providers;
}

describe("Grok subscription transport", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("sends explicit vault identity, target and selected model, without a default-account request", async () => {
    invoke.mockResolvedValue(result);
    await expect((await ports()).bindXaiManaged(request)).resolves.toEqual(
      result,
    );
    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "bind_xai_managed_provider",
      { request },
    );
  });

  it.each([
    { app: "claude", accountId: XAI_ACCOUNT_ID },
    { ...request, accountId: null },
    { ...request, accountId: "legacy-xai-account" },
    { ...request, app: "workbuddy" },
    { ...request, modelId: "" },
    { ...request, modelId: "grok\nother" },
    { ...request, modelId: "x".repeat(129) },
    { ...request, modelId: "../model" },
    { ...request, token: "SENTINEL-SECRET" },
  ])(
    "rejects invalid or excess request data before IPC %#",
    async (invalid) => {
      await expect(
        (await ports()).bindXaiManaged(
          invalid as unknown as BindXaiManagedRequest,
        ),
      ).rejects.toThrow("SuperGrok bind request is invalid");
      expect(invoke).not.toHaveBeenCalled();
    },
  );

  it.each([
    null,
    { ...result, app: "codex" },
    { ...result, activated: false },
    { ...result, providerId: "" },
    { ...result, providerName: "\n" },
    { ...result, accessToken: "SENTINEL-SECRET" },
  ])(
    "fails closed on invalid, cross-target or secret-bearing results %#",
    async (invalid) => {
      invoke.mockResolvedValue(invalid);
      await expect((await ports()).bindXaiManaged(request)).rejects.toEqual({
        code: "rollback_partial_state_unknown",
      });
    },
  );

  it("requires Codex and Desktop to remain drafts", async () => {
    const providerPorts = await ports();
    for (const app of ["codex", "claude-desktop"] as const) {
      invoke.mockResolvedValue({ ...result, app, activated: false });
      await expect(
        providerPorts.bindXaiManaged({ ...request, app }),
      ).resolves.toMatchObject({ app, activated: false });
      invoke.mockResolvedValue({ ...result, app, activated: true });
      await expect(
        providerPorts.bindXaiManaged({ ...request, app }),
      ).rejects.toEqual({ code: "rollback_partial_state_unknown" });
    }
  });

  it("retains only closed native failure codes", async () => {
    invoke.mockRejectedValue({ code: "account_unavailable" });
    await expect((await ports()).bindXaiManaged(request)).rejects.toEqual({
      code: "account_unavailable",
    });
    invoke.mockRejectedValue({
      code: "account_unavailable",
      token: "SENTINEL-SECRET",
    });
    await expect((await ports()).bindXaiManaged(request)).rejects.toEqual({
      code: "rollback_partial_state_unknown",
    });
  });

  it("fetches models using the chosen vault identity and rejects malformed IDs", async () => {
    const providerPorts = await ports();
    invoke.mockResolvedValue([
      { id: "grok-fixture-1", owned_by: "xai" },
      "grok-fixture-2",
      "grok-fixture-1",
    ]);
    await expect(
      providerPorts.fetchXaiManagedModels(XAI_ACCOUNT_ID),
    ).resolves.toEqual({
      models: ["grok-fixture-1", "grok-fixture-2"],
      truncated: false,
    });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("get_xai_oauth_models", {
      accountId: XAI_ACCOUNT_ID,
    });
    invoke.mockReset();
    await expect(providerPorts.fetchXaiManagedModels("")).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
    invoke.mockResolvedValue([{ id: "bad\nmodel" }]);
    await expect(
      providerPorts.fetchXaiManagedModels(XAI_ACCOUNT_ID),
    ).rejects.toThrow("xAI models are unavailable");
  });

  it("does not invent native subscription operations in a normal browser", async () => {
    const browser = createBrowserFeaturePorts();
    await expect(browser.providers.bindXaiManaged(request)).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(
      browser.providers.fetchXaiManagedModels(XAI_ACCOUNT_ID),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
  });
});
