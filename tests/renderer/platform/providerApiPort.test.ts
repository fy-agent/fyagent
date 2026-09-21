import { beforeEach, describe, expect, it, vi } from "vitest";
import { createModelFeaturePorts } from "@/shared/platform/tauri/feature-ports/models";
import { createChangePlansPort } from "@/shared/platform/tauri/feature-ports/changePlans";
import type { ProviderQuickSetupRequest } from "@/shared/features/models";
import { changePlanUpsertWire } from "../fixtures/changePlans";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const request: ProviderQuickSetupRequest = {
  name: "Chat API",
  baseUrl: "https://example.test/v1",
  apiKey: "fixture-key",
  modelId: "model-a",
  protocol: "chat",
};
const connection = {
  baseUrl: request.baseUrl,
  modelId: request.modelId,
  protocol: "chat",
};
const summary = (value: unknown) => ({
  providers: {
    saved: {
      id: "saved",
      name: "Saved Chat",
      writeTargets: [],
      connection: value,
    },
  },
  currentId: "saved",
  writeTargets: [],
});

describe("API protocol native boundary", () => {
  beforeEach(() => invoke.mockReset());

  it("preserves Chat through native quick setup and Change Plan admission", async () => {
    invoke.mockResolvedValue(changePlanUpsertWire);
    await createModelFeaturePorts().providers.applyQuickSetupWithResult(
      request,
      "codex",
    );
    await createChangePlansPort().createCodexProviderUpsertPlan(request);
    expect(invoke.mock.calls).toEqual([
      ["apply_provider_quick_setup_with_result", { request, app: "codex" }],
      ["create_codex_provider_upsert_plan", { request }],
    ]);
  });

  it.each(["anthropic", "unknown", "", null])(
    "rejects incompatible protocol %s before invocation",
    async (protocol) => {
      const invalid = {
        ...request,
        protocol,
      } as unknown as ProviderQuickSetupRequest;
      await expect(
        createChangePlansPort().createCodexProviderUpsertPlan(invalid),
      ).rejects.toThrow("invalid");
      expect(() =>
        createModelFeaturePorts().providers.applyQuickSetupWithResult(
          invalid,
          "codex",
        ),
      ).toThrow("invalid");
      expect(invoke).not.toHaveBeenCalled();
    },
  );

  it("reads exactly public connection fields and rejects unknown credential members", async () => {
    invoke.mockResolvedValue(summary(connection));
    expect(
      (await createModelFeaturePorts().providers.getSummary("codex")).providers
        .saved.connection,
    ).toEqual(connection);
    for (const invalid of [
      { ...connection, apiKey: "fixture-key" },
      { ...connection, credentialRef: "secretref" },
      { ...connection, protocol: "unknown" },
      { ...connection, baseUrl: "https://user:secret@example.test" },
    ]) {
      invoke.mockResolvedValue(summary(invalid));
      await expect(
        createModelFeaturePorts().providers.getSummary("codex"),
      ).rejects.toThrow("public summary");
    }
  });

  it("forwards the selected model-probe protocol with a closed request", async () => {
    invoke.mockResolvedValue({
      success: true,
      status: "operational",
      message: "OK",
      responseTimeMs: 1,
      httpStatus: 200,
      modelUsed: "model-a",
      testedAt: 1,
      retryCount: 0,
    });
    await createModelFeaturePorts().providers.checkModel({
      app: "codex",
      baseUrl: request.baseUrl,
      apiKey: request.apiKey,
      modelId: "model-a",
      protocol: "chat",
    });
    expect(invoke).toHaveBeenCalledWith("stream_check_model", {
      app: "codex",
      baseUrl: request.baseUrl,
      apiKey: request.apiKey,
      modelId: "model-a",
      protocol: "chat",
      codexImageExtension: undefined,
    });
  });
});
