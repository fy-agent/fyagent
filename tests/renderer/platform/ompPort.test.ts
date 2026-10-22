import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  OmpPortError,
  classifyOmpNativeError,
  ompErrorCode,
  type OmpApplyConfirmation,
  type OmpSaveRequest,
} from "@/shared/features/omp";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { createTauriFeaturePorts } from "@/shared/platform/tauri/features";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const provider = {
  id: "fyagent-gateway",
  baseUrl: "https://gateway.example/v1",
  api: "openai-completions",
  auth: null,
  hasApiKey: true,
  hasCommandKey: false,
  models: [{ id: "vendor/model-a", name: null }],
};

const readWire = {
  profile: "default",
  modelsPath: "~/.omp/agent/models.yml",
  configPath: "~/.omp/agent/config.yml",
  modelsRevision: "models-rev",
  configRevision: "missing",
  providers: [provider],
  defaultModel: null,
  modelsFileStatus: "ok",
  configFileStatus: "notFound",
};

const request: OmpSaveRequest = {
  provider: {
    id: "fyagent-gateway",
    baseUrl: "https://gateway.example/v1",
    api: "openai-completions",
    apiKey: "GATEWAY_KEY",
    auth: null,
    models: [{ id: "vendor/model-a", name: null }],
  },
  defaultModel: "fyagent-gateway/vendor/model-a",
  expectedModelsRevision: "models-rev",
  expectedConfigRevision: "missing",
};

const previewWire = {
  planId: "omp-plan:1",
  planDigest: "digest-1",
  profile: "default",
  modelsPath: "~/.omp/agent/models.yml",
  configPath: "~/.omp/agent/config.yml",
  provider,
  defaultModel: "fyagent-gateway/vendor/model-a",
  changedFiles: ["models", "config"],
  warnings: [],
};

const confirmation: OmpApplyConfirmation = {
  planId: "omp-plan:1",
  planDigest: "digest-1",
};

const savedWire = {
  modelsPath: "~/.omp/agent/models.yml",
  configPath: "~/.omp/agent/config.yml",
  modelsRevision: "models-rev-2",
  configRevision: "config-rev-2",
  provider,
  defaultModel: "fyagent-gateway/vendor/model-a",
  modelsWritten: true,
  configWritten: true,
};

describe("omp models feature port", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("maps every method to its exact native command and payload", async () => {
    const ports = createTauriFeaturePorts();
    invoke.mockImplementation(async (command: string) => {
      switch (command) {
        case "read_omp_config":
          return readWire;
        case "preview_omp_save":
          return previewWire;
        case "apply_omp_save":
          return savedWire;
        case "discover_omp_cli":
          return {
            detected: true,
            version: "18.12.5",
            executablePath: "C:\\Users\\me\\.omp\\bin\\omp.exe",
          };
        default:
          throw new Error(command);
      }
    });

    await expect(ports.ompModels.readConfig()).resolves.toMatchObject({
      profile: "default",
      configFileStatus: "notFound",
    });
    await expect(ports.ompModels.previewSave(request)).resolves.toEqual(
      previewWire,
    );
    await expect(ports.ompModels.applySave(confirmation)).resolves.toEqual(
      savedWire,
    );
    await expect(ports.ompModels.discoverCli()).resolves.toEqual({
      detected: true,
      version: "18.12.5",
      executablePath: "C:\\Users\\me\\.omp\\bin\\omp.exe",
    });
    expect(invoke.mock.calls).toEqual([
      ["read_omp_config"],
      ["preview_omp_save", { plan: request }],
      [
        "apply_omp_save",
        { plan: { planId: "omp-plan:1", planDigest: "digest-1" } },
      ],
      ["discover_omp_cli"],
    ]);
  });

  it("never forwards the key intent on apply", async () => {
    const ports = createTauriFeaturePorts();
    invoke.mockResolvedValueOnce(savedWire);
    await ports.ompModels.applySave({
      ...confirmation,
      ...({ provider: request.provider } as object),
    } as OmpApplyConfirmation);
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("GATEWAY_KEY");
  });

  it("rejects extra, missing, or unknown native fields", async () => {
    const ports = createTauriFeaturePorts();
    for (const wire of [
      { ...readWire, extra: true },
      { ...readWire, modelsFileStatus: "corrupt" },
      { ...readWire, providers: [{ ...provider, apiKey: "secret" }] },
      { ...readWire, providers: [provider, provider] },
      {
        ...readWire,
        providers: [{ ...provider, hasApiKey: false, hasCommandKey: true }],
      },
    ]) {
      invoke.mockResolvedValueOnce(wire);
      await expect(ports.ompModels.readConfig()).rejects.toMatchObject({
        code: "unknown",
      });
    }
    for (const wire of [
      { ...savedWire, outcome: "saved" },
      { ...savedWire, recovery: null },
      { ...savedWire, modelsWritten: "yes" },
    ]) {
      invoke.mockResolvedValueOnce(wire);
      await expect(
        ports.ompModels.applySave(confirmation),
      ).rejects.toBeInstanceOf(OmpPortError);
    }
  });

  it("binds the preview to the submitted provider and default intent", async () => {
    const ports = createTauriFeaturePorts();
    for (const wire of [
      { ...previewWire, provider: { ...provider, id: "other" } },
      {
        ...previewWire,
        provider: { ...provider, baseUrl: "https://other.example/v1" },
      },
      { ...previewWire, provider: { ...provider, models: [] } },
      { ...previewWire, defaultModel: null },
      { ...previewWire, changedFiles: ["models", "models"] },
      { ...previewWire, changedFiles: ["everything"] },
      { ...previewWire, warnings: [1] },
      { ...previewWire, planRevision: "legacy" },
    ]) {
      invoke.mockResolvedValueOnce(wire);
      await expect(ports.ompModels.previewSave(request)).rejects.toBeInstanceOf(
        OmpPortError,
      );
    }
  });

  it("keeps the existing default when the request omits it", async () => {
    const ports = createTauriFeaturePorts();
    invoke.mockResolvedValueOnce({
      ...previewWire,
      defaultModel: "other/model",
      changedFiles: ["models"],
      warnings: ["已有模型会保留"],
    });
    await expect(
      ports.ompModels.previewSave({ ...request, defaultModel: null }),
    ).resolves.toMatchObject({
      defaultModel: "other/model",
      changedFiles: ["models"],
      warnings: ["已有模型会保留"],
    });
  });

  it("classifies native errors into closed codes without raw text", async () => {
    const ports = createTauriFeaturePorts();
    const cases: [string, string][] = [
      ["并发冲突: omp models.yml changed since the preview", "stale"],
      ["无效输入: unsupported omp provider api 'x'", "invalid_input"],
      ["配置错误: omp models YAML parse failed: line 3", "config"],
      ["IO 错误: C:\\secret: denied", "unknown"],
    ];
    for (const [message, code] of cases) {
      expect(classifyOmpNativeError(message)).toBe(code);
      invoke.mockRejectedValueOnce(message);
      const error = await ports.ompModels
        .readConfig()
        .catch((cause: unknown) => cause);
      expect(ompErrorCode(error)).toBe(code);
      expect(String((error as Error).message)).not.toContain("secret");
    }
  });

  it("is native-only in the browser preview", async () => {
    const ports = createBrowserFeaturePorts();
    await expect(ports.ompModels.readConfig()).rejects.toThrow();
    await expect(ports.ompModels.discoverCli()).rejects.toThrow();
  });

  it("rejects a version without a detected CLI but keeps a found path", async () => {
    const ports = createTauriFeaturePorts();
    invoke.mockResolvedValueOnce({
      detected: false,
      version: "1.0.0",
      executablePath: null,
    });
    await expect(ports.ompModels.discoverCli()).rejects.toBeInstanceOf(
      OmpPortError,
    );
    invoke.mockResolvedValueOnce({
      detected: false,
      version: null,
      executablePath: "C:\\tools\\omp.exe",
    });
    await expect(ports.ompModels.discoverCli()).resolves.toEqual({
      detected: false,
      version: null,
      executablePath: "C:\\tools\\omp.exe",
    });
  });
});
