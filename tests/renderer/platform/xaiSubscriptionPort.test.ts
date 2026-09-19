import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  BindManagedProxyRequest,
  BindOpenCodeManagedRequest,
  BindXaiManagedRequest,
} from "@/shared/features/models";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/shared/platform/browser/features";
import { OPENAI_ACCOUNT_ID, XAI_ACCOUNT_ID } from "../fixtures/managedAuth";

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

async function openCodePorts() {
  const { createModelFeaturePorts } = await import(
    "@/shared/platform/tauri/feature-ports/models"
  );
  return createModelFeaturePorts().opencodeModels;
}

describe("OpenCode subscription transport", () => {
  const submitted: BindOpenCodeManagedRequest = {
    accountId: OPENAI_ACCOUNT_ID,
    modelId: "chatgpt-fixture-model",
    expectedRevision: "revision-before",
  };
  beforeEach(() => {
    invoke.mockReset();
  });

  it("restores only OpenCode through the existing native takeover owner", async () => {
    invoke.mockResolvedValue(null);
    await expect(
      (await openCodePorts()).restoreManagedProxy(),
    ).resolves.toBeUndefined();
    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "set_proxy_takeover_for_app",
      {
        appType: "opencode",
        enabled: false,
      },
    );
    await expect(
      createBrowserFeaturePorts().opencodeModels.restoreManagedProxy(),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
  });

  it.each([undefined, true, { restored: true }])(
    "rejects unconfirmed restore results %#",
    async (response) => {
      invoke.mockResolvedValue(response);
      await expect(
        (await openCodePorts()).restoreManagedProxy(),
      ).rejects.toThrow("unconfirmed");
    },
  );

  it.each(["owned/model-1", null, "other/model-1", "SENTINEL-SECRET", 5])(
    "validates the selected model against the public snapshot %#",
    async (selectedModel) => {
      const snapshot = {
        providers: [
          { id: "owned", name: "Saved provider", modelIds: ["model-1"] },
        ],
        selectedModel,
        path: "~/.config/opencode/opencode.json",
        backupPath: "~/.config/opencode/opencode.json.backup",
        revision: "revision-before",
        exists: true,
      };
      invoke.mockResolvedValue(snapshot);
      const read = (await openCodePorts()).getSnapshot();
      if (selectedModel === null || selectedModel === "owned/model-1")
        await expect(read).resolves.toEqual(snapshot);
      else await expect(read).rejects.toThrow("selected model is unavailable");
    },
  );

  it.each([null, "revision-before"])(
    "sends only explicit account/model and the confirmed config revision %s",
    async (expectedRevision) => {
      const response = { ...result, app: "opencode" };
      invoke.mockResolvedValue(response);
      const request = { ...submitted, expectedRevision };
      await expect(
        (await openCodePorts()).bindManagedProxy(request),
      ).resolves.toEqual(response);
      expect(invoke).toHaveBeenCalledExactlyOnceWith(
        "bind_opencode_managed_proxy",
        { request },
      );
    },
  );

  it.each([
    { ...submitted, expectedRevision: undefined },
    { ...submitted, expectedRevision: 1 },
    { ...submitted, expectedRevision: "" },
    { ...submitted, accountId: "legacy-account" },
    { ...submitted, modelId: "../invalid" },
    { ...submitted, app: "codex" },
    { ...submitted, token: "SENTINEL-SECRET" },
  ])("rejects invalid OpenCode requests before IPC %#", async (request) => {
    await expect(
      (await openCodePorts()).bindManagedProxy(
        request as unknown as BindOpenCodeManagedRequest,
      ),
    ).rejects.toThrow("Subscription bind request is invalid");
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([
    { ...result, app: "codex" },
    { ...result, app: "opencode", activated: false },
    { ...result, app: "opencode", accessToken: "SENTINEL-SECRET" },
    null,
  ])("rejects mismatched or unconfirmed results %#", async (response) => {
    invoke.mockResolvedValue(response);
    await expect(
      (await openCodePorts()).bindManagedProxy(submitted),
    ).rejects.toEqual({ code: "rollback_partial_state_unknown" });
  });

  it("preserves revision conflict and keeps the browser native-only", async () => {
    invoke.mockRejectedValue({ code: "provider_conflict" });
    await expect(
      (await openCodePorts()).bindManagedProxy(submitted),
    ).rejects.toEqual({ code: "provider_conflict" });
    await expect(
      createBrowserFeaturePorts().opencodeModels.bindManagedProxy(submitted),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
  });
});

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
      ).rejects.toThrow("Subscription bind request is invalid");
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
    await expect(
      browser.providers.bindManagedProxy({ ...request, app: "grokbuild" }),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
  });

  it.each(["claude", "codex", "grokbuild"] as const)(
    "uses the shared command with strict %s activation semantics for either account identity",
    async (app) => {
      const providerPorts = await ports();
      for (const accountId of [OPENAI_ACCOUNT_ID, XAI_ACCOUNT_ID]) {
        invoke.mockReset();
        const submitted = { ...request, app, accountId };
        const returned = { ...result, app, activated: app !== "codex" };
        invoke.mockResolvedValue(returned);
        await expect(
          providerPorts.bindManagedProxy(submitted),
        ).resolves.toEqual(returned);
        expect(invoke).toHaveBeenCalledExactlyOnceWith(
          "bind_managed_proxy_provider",
          { request: submitted },
        );
        invoke.mockResolvedValue({
          ...returned,
          activated: !returned.activated,
        });
        await expect(providerPorts.bindManagedProxy(submitted)).rejects.toEqual(
          { code: "rollback_partial_state_unknown" },
        );
      }
    },
  );

  it.each([
    { ...request, app: "claude-desktop" },
    { ...request, app: "workbuddy" },
    { ...request, accountId: "default" },
    { ...request, token: "SENTINEL-SECRET" },
    { ...request, modelId: "bad\nmodel" },
  ])("rejects out-of-scope generic requests before IPC %#", async (invalid) => {
    await expect(
      (await ports()).bindManagedProxy(
        invalid as unknown as BindManagedProxyRequest,
      ),
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([
    null,
    { ...result, app: "grokbuild" },
    { ...result, token: "SENTINEL-SECRET" },
  ])("rejects malformed generic results %#", async (invalid) => {
    invoke.mockResolvedValue(invalid);
    await expect(
      (await ports()).bindManagedProxy({ ...request, app: "claude" }),
    ).rejects.toEqual({ code: "rollback_partial_state_unknown" });
  });
});
