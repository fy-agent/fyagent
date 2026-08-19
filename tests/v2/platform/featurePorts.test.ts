import { beforeEach, describe, expect, it, vi } from "vitest";

import { CODEX_DESKTOP_PAYLOAD_ERROR } from "@/shared/codex-desktop";
import type {
  InstallerErrorDto,
  JobSnapshot,
  LocalInstallStatus,
  RemoteReleaseStatus,
} from "@/shared/codex-desktop";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/v2/shared/platform/browser/features";
import { PROMPT_APP_IDS } from "@/v2/shared/features/types";
import type {
  AgentCapabilityId,
  AgentCatalogEntry,
  AgentCatalogId,
  AgentCatalogResult,
  HermesMemoryKind,
  ManagedPrompt,
  MemoryDocumentId,
  PromptAppId,
  QoderWorkHooksSnapshot,
  SaveQoderWorkHooksRequest,
} from "@/v2/shared/features/types";

const invoke = vi.fn();
const listen = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const installerReleaseId = `v1:${"a".repeat(64)}`;
const installerRemote: RemoteReleaseStatus = {
  releaseId: installerReleaseId,
  displayVersion: "26.814.1000",
  platformVersion: {
    kind: "windows_msix",
    major: 26,
    minor: 814,
    build: 1000,
    revision: 0,
  },
  downloadSizeHint: 1_048_576,
  checkedAt: "2026-08-14T05:00:00Z",
};
const installerLocal: LocalInstallStatus = {
  state: "not_installed",
  platform: "windows",
  architecture: "x86_64",
};

function installerError(): InstallerErrorDto {
  return {
    code: "DOWNLOAD_FAILED",
    stage: "downloading",
    messageKey: "codexDesktop.error.downloadFailed",
    retryable: true,
    suggestedAction: "retry",
    details: {
      endpointKind: "artifact",
      attempt: 1,
      maxAttempts: 3,
      httpStatus: 503,
      platformErrorCode: null,
      redactedMessage: "Fixture download failed",
      context: { operation: "download" },
    },
  };
}

function installerJob(
  stage: JobSnapshot["stage"] = "checking",
  sequence = 0,
): JobSnapshot {
  return {
    jobId: "fixture-job-001",
    sequence,
    stage,
    release: installerRemote,
    startedAt: "2026-08-14T05:00:01Z",
    updatedAt: "2026-08-14T05:00:02Z",
    progress: null,
    cancellable: stage === "checking",
    result: null,
    error: stage === "failed" ? installerError() : null,
  };
}

const agentCapabilityIds: readonly AgentCapabilityId[] = [
  "product.open",
  "app.detect",
  "app.launch",
  "skills.read",
  "skills.write",
  "hooks.read",
  "hooks.write",
  "models.validate",
  "models.write",
  "mcp.validate",
  "mcp.write",
];

const agentVariantById = {
  qoderwork: "qoderwork-cn",
  "trae-work": "trae-work-cn",
  workbuddy: "workbuddy",
  codex: "codex",
  "claude-code": "claude-code",
  opencode: "opencode",
} as const;

function catalogEntry(
  id: AgentCatalogId,
  displayName: string,
  officialLinks: AgentCatalogEntry["officialLinks"],
): AgentCatalogEntry {
  return {
    id,
    variantId: agentVariantById[id],
    displayName,
    description: `${displayName} catalog fixture`,
    officialLinks,
    capabilities: agentCapabilityIds.map((capabilityId) => ({
      id: capabilityId,
      mode:
        capabilityId === "product.open" && id === "codex"
          ? "unsupported"
          : capabilityId === "app.detect" || capabilityId === "app.launch"
            ? "unverified"
            : "direct",
      reasonCode:
        capabilityId === "product.open" && id === "codex"
          ? "no_catalog_product_link"
          : capabilityId === "app.detect" || capabilityId === "app.launch"
            ? "trusted_runtime_identity_unavailable"
            : "dedicated_native_contract",
      evidenceIds: ["p0_scope"],
    })),
  };
}

function catalogFixture(): AgentCatalogResult {
  return {
    contractVersion: 3,
    reviewedAt: "2026-08-18",
    agents: [
      catalogEntry("qoderwork", "QoderWork CN", [
        {
          id: "product",
          label: "打开 QoderWork 官方页面",
          url: "https://qoder.com.cn/qoderwork",
        },
      ]),
      catalogEntry("trae-work", "TRAE Work CN", [
        {
          id: "product",
          label: "打开 TRAE Work CN 官方页面",
          url: "https://www.trae.cn/sem-work",
        },
      ]),
      catalogEntry("workbuddy", "WorkBuddy", [
        {
          id: "product",
          label: "打开 WorkBuddy 官方页面",
          url: "https://www.workbuddy.cn/",
        },
      ]),
      catalogEntry("codex", "Codex", []),
      catalogEntry("claude-code", "Claude Code", [
        {
          id: "cli",
          label: "Claude Code CLI",
          url: "https://docs.anthropic.com/en/docs/claude-code/getting-started",
        },
        {
          id: "desktop",
          label: "Claude Desktop",
          url: "https://claude.com/download",
        },
      ]),
      catalogEntry("opencode", "OpenCode", [
        {
          id: "product",
          label: "打开 OpenCode 官方页面",
          url: "https://opencode.ai",
        },
        {
          id: "cli",
          label: "OpenCode CLI",
          url: "https://opencode.ai/docs/cli",
        },
      ]),
    ],
  };
}

describe("V2 feature ports", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
  });

  it("partitions Prompt and Memory query keys by authoritative resource", async () => {
    const { featureKeys } = await import("@/v2/shared/features/queries");
    expect(featureKeys.prompts("claude")).toEqual([
      "v2",
      "prompts",
      "claude",
      "list",
    ]);
    expect(featureKeys.prompts("codex")).not.toEqual(
      featureKeys.prompts("claude"),
    );
    expect(featureKeys.promptLiveFile("claude")).toEqual([
      "v2",
      "prompts",
      "claude",
      "live-file",
    ]);
    expect(featureKeys.memoryDocument("openclaw-memory")).not.toEqual(
      featureKeys.memoryDocument("hermes-memory"),
    );
    expect(featureKeys.dailyMemoryFile("2026-08-14.md")).toEqual([
      "v2",
      "memory",
      "daily",
      "file",
      "2026-08-14.md",
    ]);
    expect(featureKeys.dailyMemorySearch("release")).toEqual([
      "v2",
      "memory",
      "daily",
      "search",
      "release",
    ]);
  });

  it("keeps native observations unavailable in browsers and rejects writes", async () => {
    const ports = createBrowserFeaturePorts();
    await expect(ports.catalog.get()).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(ports.externalAgents.getStatus("qoderwork")).resolves.toEqual({
      agentId: "qoderwork",
      detected: null,
      running: null,
      version: null,
      installSource: null,
      capabilities: [
        {
          id: "app.detect",
          state: "unverified",
          reasonCode: "trusted_runtime_identity_unavailable",
        },
        {
          id: "app.launch",
          state: "unverified",
          reasonCode: "trusted_runtime_identity_unavailable",
        },
      ],
    });
    await expect(
      ports.externalAgents.launch("qoderwork", "home"),
    ).resolves.toMatchObject({ state: "unverified" });
    await expect(ports.qoderwork.getHooks()).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(
      ports.externalMcp.validate("qoderwork", { mcpServers: {} }),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(
      ports.traeWork.validateModelConfig({
        apiFormat: "openai_chat_completions",
        urlMode: "base_url",
        url: "https://example.test/v1",
        modelId: "model-a",
        apiKey: "secret",
        allowNoApiKey: false,
        allowLoopback: false,
        allowPrivateNetwork: false,
      }),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(ports.traeWork.getModelIds()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.opencodeModels.getSnapshot()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.codexDesktop.getLocalStatus()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.codexDesktop.checkLatest(false)).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.codexDesktop.getJob()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(
      ports.codexDesktop.startInstall(installerReleaseId),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(
      ports.codexDesktop.cancelInstall("fixture-job-001"),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(ports.codexDesktop.launch()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.codexDesktop.openLogDirectory()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(
      ports.codexDesktop.subscribeJobUpdates(vi.fn()),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(ports.providers.getSummary("codex")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.workbuddy.getStatus()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.workbuddy.getModelIds()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.skills.getInstalled()).resolves.toEqual([]);
    await expect(ports.mcp.getAll()).resolves.toEqual({});
    await expect(ports.prompts.getAll("claude")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.prompts.getCurrentFileContent("codex")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(
      ports.prompts.upsert("gemini", {
        id: "prompt-a",
        name: "Prompt A",
        content: "content",
        enabled: false,
      }),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(ports.prompts.delete("grokbuild", "prompt-a")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.prompts.enable("opencode", "prompt-a")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.prompts.importFromFile("openclaw")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.memory.readDocument("openclaw-memory")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.memory.getHermesLimits()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.memory.listDailyFiles()).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.memory.searchDailyFiles("release")).rejects.toThrow(
      NATIVE_ONLY_ERROR,
    );
    await expect(ports.settings.get()).resolves.toEqual({});
    await expect(
      ports.providers.applyQuickSetupWithResult(
        {
          name: "Draft",
          baseUrl: "https://example.test/v1",
          apiKey: "key",
          modelId: "model",
        },
        "codex",
      ),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(
      ports.workbuddy.fetchModels({
        baseUrl: "https://example.test/v1",
        apiKey: "test-key",
        allowNoApiKey: false,
      }),
    ).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(ports.mcp.importFromApps()).rejects.toThrow(NATIVE_ONLY_ERROR);
  });

  it("uses exact Agent, Provider, and WorkBuddy commands and validates Provider summaries", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const sentinelSecret = "SENTINEL-PROVIDER-SECRET";
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_agent_catalog") return catalogFixture();
      if (command === "get_provider_summary") {
        return {
          providers: {
            "provider-a": {
              id: "provider-a",
              name: "Provider A",
            },
          },
          currentId: "provider-a",
        };
      }
      if (command === "get_workbuddy_status") {
        return {
          path: "~/.workbuddy/models.json",
          exists: true,
          modelCount: 1,
          revision: "opaque-revision",
          backupExists: false,
          format: "legacyArray",
        };
      }
      if (command === "get_workbuddy_model_ids") {
        return { ids: ["model-a"], revision: "opaque-revision" };
      }
      if (command === "fetch_workbuddy_models") {
        return { models: ["model-a"], truncated: false };
      }
      if (command === "save_workbuddy_models") {
        return {
          state: "saved",
          revision: "next-revision",
          modelCount: 1,
          createdEntries: 1,
          updatedEntries: 0,
        };
      }
      return {
        value: { warnings: [] },
        liveConfigChanged: false,
        app: "codex",
      };
    });

    const ports = createTauriFeaturePorts();
    const request = {
      name: "Quick setup",
      baseUrl: "https://example.test/v1",
      apiKey: "mutation-only-key",
      modelId: "model-a",
    };
    const fetchRequest = {
      baseUrl: "https://example.test/v1",
      apiKey: "workbuddy-key",
      allowNoApiKey: false,
    };
    const saveRequest = {
      ...fetchRequest,
      selectedModelIds: ["model-a"],
      manualModelIds: [],
      clearExistingApiKeys: false,
      expectedRevision: "opaque-revision",
      overwriteToken: "opaque-token",
    };

    await ports.catalog.get();
    const summary = await ports.providers.getSummary("codex");
    await ports.providers.applyQuickSetupWithResult(request, "codex");
    await ports.workbuddy.getStatus();
    await ports.workbuddy.getModelIds();
    await ports.workbuddy.fetchModels(fetchRequest);
    await ports.workbuddy.saveModels(saveRequest);

    expect(summary.providers).toEqual({
      "provider-a": {
        id: "provider-a",
        name: "Provider A",
      },
    });
    expect(JSON.stringify(summary)).not.toContain(sentinelSecret);
    expect(summary.providers["provider-a"]).not.toHaveProperty(
      "settingsConfig",
    );
    expect(invoke.mock.calls).toEqual([
      ["get_agent_catalog"],
      ["get_provider_summary", { app: "codex" }],
      ["apply_provider_quick_setup_with_result", { request, app: "codex" }],
      ["get_workbuddy_status"],
      ["get_workbuddy_model_ids"],
      ["fetch_workbuddy_models", { request: fetchRequest }],
      ["save_workbuddy_models", { request: saveRequest }],
    ]);
  });

  it("uses exact TRAE observation and OpenCode model commands", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_traework_model_ids") {
        return {
          modelIds: ["model-a"],
          revision: "trae-rev",
          truncated: false,
        };
      }
      if (command === "get_opencode_model_snapshot") {
        return {
          providers: [
            { id: "gateway", name: "Gateway", modelIds: ["model-a"] },
          ],
          revision: "oc-rev",
        };
      }
      if (command === "fetch_opencode_provider_models") {
        return { models: [{ id: "model-a" }], truncated: false };
      }
      if (command === "save_opencode_models") {
        return {
          state: "saved",
          revision: "oc-rev-2",
          modelCount: 1,
          createdEntries: 1,
          updatedEntries: 0,
        };
      }
      throw new Error(`unexpected command ${command}`);
    });

    const ports = createTauriFeaturePorts();
    const openCodeFetch = {
      baseUrl: "https://example.test/v1",
      apiKey: "oc-key",
      allowNoApiKey: false,
    };

    await expect(ports.traeWork.getModelIds()).resolves.toEqual({
      modelIds: ["model-a"],
      revision: "trae-rev",
      truncated: false,
    });
    await expect(ports.opencodeModels.getSnapshot()).resolves.toEqual({
      providers: [{ id: "gateway", name: "Gateway", modelIds: ["model-a"] }],
      revision: "oc-rev",
    });
    await ports.opencodeModels.fetchProviderModels(openCodeFetch);
    await ports.opencodeModels.saveModels({
      providerName: "Gateway",
      baseUrl: openCodeFetch.baseUrl,
      apiKey: openCodeFetch.apiKey,
      selectedModelIds: ["model-a"],
      expectedRevision: "oc-rev",
    });

    expect(invoke.mock.calls).toEqual([
      ["get_traework_model_ids"],
      ["get_opencode_model_snapshot"],
      ["fetch_opencode_provider_models", { request: openCodeFetch }],
      [
        "save_opencode_models",
        {
          request: {
            providerName: "Gateway",
            baseUrl: openCodeFetch.baseUrl,
            apiKey: openCodeFetch.apiKey,
            selectedModelIds: ["model-a"],
            expectedRevision: "oc-rev",
          },
        },
      ],
    ]);
  });

  it("rejects a Provider map whose key and public ID disagree", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    invoke.mockResolvedValue({
      providers: {
        "provider-map-key": {
          id: "different-provider-id",
          name: "Mismatched Provider",
        },
      },
      currentId: "provider-map-key",
    });

    await expect(
      createTauriFeaturePorts().providers.getSummary("codex"),
    ).rejects.toThrow("Provider public summary is unavailable");
  });

  it("decodes only the exact Agent catalog v3 wire contract", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const ports = createTauriFeaturePorts();
    const expected = catalogFixture();
    invoke.mockResolvedValueOnce(expected);
    await expect(ports.catalog.get()).resolves.toEqual(expected);

    const invalidPayloads: unknown[] = [];

    const legacy = structuredClone(expected);
    Object.assign(legacy, { contractVersion: 2 });
    invalidPayloads.push(legacy);

    const future = structuredClone(expected);
    Object.assign(future, { contractVersion: 4 });
    invalidPayloads.push(future);

    const invalidDate = structuredClone(expected);
    Object.assign(invalidDate, { reviewedAt: "2026-02-30" });
    invalidPayloads.push(invalidDate);

    const extraTopLevelKey = structuredClone(expected);
    Object.assign(extraTopLevelKey, { officialUrl: "https://example.test" });
    invalidPayloads.push(extraTopLevelKey);

    const wrongAgentOrder = structuredClone(expected);
    wrongAgentOrder.agents.reverse();
    invalidPayloads.push(wrongAgentOrder);

    const unknownVariant = structuredClone(expected);
    Object.assign(unknownVariant.agents[0], { variantId: "qoderwork-global" });
    invalidPayloads.push(unknownVariant);

    const unknownCapabilityMode = structuredClone(expected);
    Object.assign(unknownCapabilityMode.agents[0].capabilities[0], {
      mode: "available",
    });
    invalidPayloads.push(unknownCapabilityMode);

    const unknownCapability = structuredClone(expected);
    Object.assign(unknownCapability.agents[0].capabilities[0], {
      id: "models.execute",
    });
    invalidPayloads.push(unknownCapability);

    const unknownReason = structuredClone(expected);
    Object.assign(unknownReason.agents[0].capabilities[0], {
      reasonCode: "legacy_reason",
    });
    invalidPayloads.push(unknownReason);

    const unknownEvidence = structuredClone(expected);
    Object.assign(unknownEvidence.agents[0].capabilities[0], {
      evidenceIds: ["unknown_evidence"],
    });
    invalidPayloads.push(unknownEvidence);

    const duplicateEvidence = structuredClone(expected);
    duplicateEvidence.agents[0].capabilities[0].evidenceIds = [
      "p0_scope",
      "p0_scope",
    ];
    invalidPayloads.push(duplicateEvidence);

    const extraEntryKey = structuredClone(expected);
    Object.assign(extraEntryKey.agents[0], { status: "legacy" });
    invalidPayloads.push(extraEntryKey);

    const emptyLabel = structuredClone(expected);
    emptyLabel.agents[0].officialLinks[0].label = "";
    invalidPayloads.push(emptyLabel);

    for (const url of [
      "http://qoder.com.cn/qoderwork",
      "https://user@qoder.com.cn/qoderwork",
      "https://qoder.com.cn/qoderwork?source=test",
      "https://qoder.com.cn/qoderwork#fragment",
    ]) {
      const invalidUrl = structuredClone(expected);
      invalidUrl.agents[0].officialLinks[0].url = url;
      invalidPayloads.push(invalidUrl);
    }

    const duplicateProductLink = structuredClone(expected);
    duplicateProductLink.agents[0].officialLinks.push({
      ...duplicateProductLink.agents[0].officialLinks[0],
    });
    invalidPayloads.push(duplicateProductLink);

    const codexExternalLink = structuredClone(expected);
    codexExternalLink.agents[3].officialLinks.push({
      id: "product",
      label: "Codex product",
      url: "https://example.test/codex",
    });
    invalidPayloads.push(codexExternalLink);

    const reversedClaudeLinks = structuredClone(expected);
    reversedClaudeLinks.agents[4].officialLinks.reverse();
    invalidPayloads.push(reversedClaudeLinks);

    for (const payload of invalidPayloads) {
      invoke.mockResolvedValueOnce(payload);
      await expect(ports.catalog.get()).rejects.toThrow(
        "Agent catalog is unavailable",
      );
    }
  });

  it("uses exact runtime and Qoder Hooks IPC and rejects excess wire fields", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const status = {
      agentId: "qoderwork",
      detected: null,
      running: null,
      version: null,
      installSource: null,
      capabilities: [
        {
          id: "app.detect",
          state: "unverified",
          reasonCode: "trusted_runtime_identity_unavailable",
        },
        {
          id: "app.launch",
          state: "unverified",
          reasonCode: "trusted_runtime_identity_unavailable",
        },
      ],
    };
    const launch = {
      agentId: "qoderwork",
      destination: "hooks",
      state: "unverified",
      reasonCode: "trusted_runtime_identity_unavailable",
    };
    const snapshot: QoderWorkHooksSnapshot = {
      revision: "opaque-revision",
      exists: true,
      groups: [
        {
          event: "PreToolUse",
          matcher: "Bash",
          hooks: [{ type: "command", command: "review-command", timeout: 30 }],
        },
      ],
      restartRequired: true,
      supportedStructure: true,
    };
    invoke
      .mockResolvedValueOnce(status)
      .mockResolvedValueOnce(launch)
      .mockResolvedValueOnce(snapshot)
      .mockResolvedValueOnce({ state: "saved", snapshot });
    const ports = createTauriFeaturePorts();
    const request: SaveQoderWorkHooksRequest = {
      expectedRevision: "opaque-revision",
      groups: snapshot.groups,
    };

    await expect(ports.externalAgents.getStatus("qoderwork")).resolves.toEqual(
      status,
    );
    await expect(
      ports.externalAgents.launch("qoderwork", "hooks"),
    ).resolves.toEqual(launch);
    await expect(ports.qoderwork.getHooks()).resolves.toEqual(snapshot);
    await expect(ports.qoderwork.saveHooks(request)).resolves.toEqual({
      state: "saved",
      snapshot,
    });
    expect(invoke.mock.calls).toEqual([
      ["get_external_agent_status", { agentId: "qoderwork" }],
      ["launch_external_agent", { agentId: "qoderwork", destination: "hooks" }],
      ["get_qoderwork_hooks"],
      ["save_qoderwork_hooks", { request }],
    ]);

    invoke.mockResolvedValueOnce({ ...status, executable: "QoderWork.exe" });
    await expect(ports.externalAgents.getStatus("qoderwork")).rejects.toThrow(
      "External agent status is unavailable",
    );
    invoke.mockResolvedValueOnce({
      ...snapshot,
      groups: [{ ...snapshot.groups[0], event: "UnknownEvent" }],
    });
    await expect(ports.qoderwork.getHooks()).rejects.toThrow(
      "QoderWork Hooks are unavailable",
    );
  });

  it("uses exact MCP and two-stage TRAE IPC while keeping validation results secret-free", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const requestId = "123e4567-e89b-42d3-a456-426614174000";
    const sentinel = "MCP-SECRET-SENTINEL-814";
    const config = {
      mcpServers: {
        demo: {
          command: "demo",
          env: { DEMO_TOKEN: sentinel },
        },
      },
    };
    const mcpResult = {
      agentId: "trae-work",
      valid: true,
      findings: [
        {
          serverId: "demo",
          transport: "stdio",
          reasonCodes: ["TRAE_MCP_SERVER_VALID"],
          executableAvailable: true,
          hasSecrets: true,
        },
      ],
      redactedTemplate: {
        mcpServers: {
          demo: { command: "demo", env: { DEMO_TOKEN: "<redacted>" } },
        },
      },
    };
    const request = {
      apiFormat: "openai_chat_completions",
      urlMode: "base_url",
      url: "https://gateway.example.test/v1",
      modelId: "model-a",
      apiKey: "short-lived-key",
      allowNoApiKey: false,
      allowLoopback: false,
      allowPrivateNetwork: false,
    } as const;
    const validation = {
      requestId,
      state: "valid",
      reasonCode: "TRAE_MODEL_CONFIG_VALID",
      durationBucket: "lt_1s",
      statusClass: null,
    };
    const probe = {
      requestId,
      state: "reachable",
      reasonCode: "TRAE_ENDPOINT_REACHABLE",
      durationBucket: "1s_to_3s",
      statusClass: "2xx",
    };
    const cancel = { requestId, cancelled: true };
    invoke
      .mockResolvedValueOnce(mcpResult)
      .mockResolvedValueOnce(validation)
      .mockResolvedValueOnce(probe)
      .mockResolvedValueOnce(cancel);
    const ports = createTauriFeaturePorts();

    const sanitized = await ports.externalMcp.validate("trae-work", config);
    await expect(ports.traeWork.validateModelConfig(request)).resolves.toEqual(
      validation,
    );
    await expect(
      ports.traeWork.testModelEndpoint(requestId, request),
    ).resolves.toEqual(probe);
    await expect(
      ports.traeWork.cancelModelEndpoint(requestId),
    ).resolves.toEqual(cancel);
    expect(JSON.stringify(sanitized)).not.toContain(sentinel);
    expect(invoke.mock.calls).toEqual([
      ["validate_external_mcp_config", { agentId: "trae-work", config }],
      ["validate_traework_model_config", { request }],
      ["test_traework_model_endpoint", { requestId, request }],
      ["cancel_traework_model_endpoint", { requestId }],
    ]);

    invoke.mockResolvedValueOnce({
      ...mcpResult,
      redactedTemplate: config,
    });
    await expect(
      ports.externalMcp.validate("trae-work", config),
    ).rejects.toThrow("External MCP validation result is unavailable");

    invoke.mockResolvedValueOnce({ ...probe, state: "future_state" });
    await expect(
      ports.traeWork.testModelEndpoint(requestId, request),
    ).rejects.toThrow("TRAE endpoint result is unavailable");
  });

  it("validates Codex Desktop results and uses only the exact installer IPC", async () => {
    const unlisten = vi.fn();
    let eventHandler: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation(
      async (
        _eventName: string,
        handler: (event: { payload: unknown }) => void,
      ) => {
        eventHandler = handler;
        return unlisten;
      },
    );
    invoke.mockImplementation(async (command: string) => {
      switch (command) {
        case "codex_desktop_get_local_status":
          return installerLocal;
        case "codex_desktop_check_latest":
          return installerRemote;
        case "codex_desktop_get_job":
          return null;
        case "codex_desktop_start_install":
          return installerJob("checking", 1);
        case "codex_desktop_cancel_install":
          return installerJob("cancelled", 2);
        case "codex_desktop_launch":
        case "codex_desktop_open_log_directory":
          return undefined;
        default:
          throw new Error(`Unexpected command: ${command}`);
      }
    });

    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const ports = createTauriFeaturePorts();
    await expect(ports.codexDesktop.getLocalStatus()).resolves.toEqual(
      installerLocal,
    );
    await expect(ports.codexDesktop.checkLatest(true)).resolves.toEqual(
      installerRemote,
    );
    await expect(ports.codexDesktop.getJob()).resolves.toBeNull();
    await expect(
      ports.codexDesktop.startInstall(installerReleaseId),
    ).resolves.toEqual(installerJob("checking", 1));
    await expect(
      ports.codexDesktop.cancelInstall("fixture-job-001"),
    ).resolves.toEqual(installerJob("cancelled", 2));
    await expect(ports.codexDesktop.launch()).resolves.toBeUndefined();
    await expect(
      ports.codexDesktop.openLogDirectory(),
    ).resolves.toBeUndefined();

    expect(invoke.mock.calls).toEqual([
      ["codex_desktop_get_local_status"],
      ["codex_desktop_check_latest", { force: true }],
      ["codex_desktop_get_job"],
      [
        "codex_desktop_start_install",
        { request: { expectedReleaseId: installerReleaseId } },
      ],
      ["codex_desktop_cancel_install", { jobId: "fixture-job-001" }],
      ["codex_desktop_launch"],
      ["codex_desktop_open_log_directory"],
    ]);
    expect(JSON.stringify(invoke.mock.calls[3])).not.toMatch(
      /url|path|hash|scope|bypass/i,
    );

    const onSnapshot = vi.fn();
    const cleanup = await ports.codexDesktop.subscribeJobUpdates(onSnapshot);
    expect(listen).toHaveBeenCalledWith(
      "codex-desktop-installer://job-updated",
      expect.any(Function),
    );
    const failedSnapshot = installerJob("failed", 3);
    eventHandler?.({ payload: failedSnapshot });
    expect(onSnapshot).toHaveBeenCalledWith(failedSnapshot);

    const invalidErrorSnapshot = structuredClone(failedSnapshot);
    Object.assign(invalidErrorSnapshot.error?.details ?? {}, {
      redactedMessage: 503,
    });
    expect(() => eventHandler?.({ payload: invalidErrorSnapshot })).toThrow(
      CODEX_DESKTOP_PAYLOAD_ERROR,
    );
    expect(onSnapshot).toHaveBeenCalledTimes(1);
    cleanup();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("rejects invalid Codex Desktop requests and payloads before React sees them", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const ports = createTauriFeaturePorts();

    await expect(
      ports.codexDesktop.startInstall("https://example.test/release.msix"),
    ).rejects.toThrow(CODEX_DESKTOP_PAYLOAD_ERROR);
    await expect(ports.codexDesktop.cancelInstall(" job-001 ")).rejects.toThrow(
      "Codex desktop installer request is invalid",
    );
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValueOnce({ ...installerLocal, unexpected: true });
    await expect(ports.codexDesktop.getLocalStatus()).rejects.toThrow(
      CODEX_DESKTOP_PAYLOAD_ERROR,
    );

    invoke.mockResolvedValueOnce({
      ...installerRemote,
      checkedAt: "2026-08-14",
    });
    await expect(ports.codexDesktop.checkLatest(false)).rejects.toThrow(
      CODEX_DESKTOP_PAYLOAD_ERROR,
    );

    invoke.mockResolvedValueOnce({
      ...installerJob("checking"),
      sequence: Number.NaN,
    });
    await expect(ports.codexDesktop.getJob()).rejects.toThrow(
      CODEX_DESKTOP_PAYLOAD_ERROR,
    );
  });

  it("uses exact existing Tauri commands and camelCase payloads", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    invoke.mockResolvedValue(undefined);
    const ports = createTauriFeaturePorts();
    const skill = {
      key: "owner/repo:skill-a",
      name: "Skill A",
      description: "A",
      directory: "skill-a",
      repoOwner: "owner",
      repoName: "repo",
      repoBranch: "main",
    };
    const repo = {
      owner: "owner",
      name: "repo",
      branch: "main",
      enabled: true,
    };
    const server = {
      id: "server-a",
      name: "Server A",
      server: { type: "stdio" as const, command: "npx" },
      apps: {
        qoderwork: false,
        "trae-work": false,
        workbuddy: false,
        codex: false,
        claude: true,
        opencode: false,
      },
    };
    const skillApps = {
      ...server.apps,
      qoderwork: false,
      "trae-work": false,
    };
    await ports.skills.getInstalled();
    await ports.skills.getBackups();
    await ports.skills.deleteBackup("backup-a");
    await ports.skills.install(skill, "claude");
    await ports.skills.uninstall("skill-a");
    await ports.skills.restoreBackup("backup-a", "opencode");
    await ports.skills.toggleApp("skill-a", "codex", true);
    await ports.skills.scanUnmanaged();
    await ports.skills.importFromApps([
      { directory: "skill-a", apps: skillApps },
    ]);
    await ports.skills.discoverPage({
      query: "",
      status: "all",
      limit: 20,
      offset: 0,
    });
    await ports.skills.checkUpdates();
    await ports.skills.update("skill-a");
    await ports.skills.migrateStorage("unified");
    await ports.skills.searchSkillsSh("react", 20, 40);
    await ports.skills.getRepos();
    await ports.skills.addRepo(repo);
    await ports.skills.removeRepo("owner", "repo");
    await ports.skills.pickZip();
    await ports.skills.installFromZip("C:/skill.zip", "workbuddy");
    await ports.mcp.getAll();
    await ports.mcp.upsert(server);
    await ports.mcp.delete("server-a");
    await ports.mcp.toggleApp("server-a", "workbuddy", false);
    await ports.mcp.importFromApps();
    await ports.settings.get();
    await ports.settings.save({ skillSyncMethod: "copy" });
    expect(invoke.mock.calls).toEqual([
      ["get_installed_skills"],
      ["get_skill_backups"],
      ["delete_skill_backup", { backupId: "backup-a" }],
      ["install_skill_unified", { skill, currentApp: "claude" }],
      ["uninstall_skill_unified", { id: "skill-a" }],
      [
        "restore_skill_backup",
        { backupId: "backup-a", currentApp: "opencode" },
      ],
      ["toggle_skill_app", { id: "skill-a", app: "codex", enabled: true }],
      ["scan_unmanaged_skills"],
      [
        "import_skills_from_apps",
        { imports: [{ directory: "skill-a", apps: skillApps }] },
      ],
      [
        "discover_available_skills_page",
        {
          query: "",
          repo: null,
          status: "all",
          limit: 20,
          offset: 0,
        },
      ],
      ["check_skill_updates"],
      ["update_skill", { id: "skill-a" }],
      ["migrate_skill_storage", { target: "unified" }],
      ["search_skills_sh", { query: "react", limit: 20, offset: 40 }],
      ["get_skill_repos"],
      ["add_skill_repo", { repo }],
      ["remove_skill_repo", { owner: "owner", name: "repo" }],
      ["open_zip_file_dialog"],
      [
        "install_skills_from_zip",
        { filePath: "C:/skill.zip", currentApp: "workbuddy" },
      ],
      ["get_mcp_servers"],
      ["upsert_mcp_server", { server }],
      ["delete_mcp_server", { id: "server-a" }],
      [
        "toggle_mcp_app",
        { serverId: "server-a", app: "workbuddy", enabled: false },
      ],
      ["import_mcp_from_apps"],
      ["get_settings"],
      ["save_settings", { settings: { skillSyncMethod: "copy" } }],
    ]);
  });

  it("uses exact Prompt commands for every supported application and parses authoritative data", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const prompt: ManagedPrompt = {
      id: "prompt-a",
      name: "Prompt A",
      content: "Keep answers concise.",
      description: "Shared fixture",
      enabled: false,
      createdAt: 1_700_000_000,
      updatedAt: 1_700_000_100,
    };
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_prompts") return { [prompt.id]: prompt };
      if (command === "get_current_prompt_file_content") return "live prompt";
      if (command === "import_prompt_from_file") return "imported-1";
      return undefined;
    });

    const ports = createTauriFeaturePorts();
    for (const app of PROMPT_APP_IDS) {
      await expect(ports.prompts.getAll(app)).resolves.toEqual([prompt]);
      await expect(ports.prompts.getCurrentFileContent(app)).resolves.toBe(
        "live prompt",
      );
      await ports.prompts.upsert(app, prompt);
      await ports.prompts.delete(app, prompt.id);
      await ports.prompts.enable(app, prompt.id);
      await expect(ports.prompts.importFromFile(app)).resolves.toBe(
        "imported-1",
      );
    }

    expect(invoke.mock.calls).toEqual(
      PROMPT_APP_IDS.flatMap((app) => [
        ["get_prompts", { app }],
        ["get_current_prompt_file_content", { app }],
        ["upsert_prompt", { app, id: prompt.id, prompt }],
        ["delete_prompt", { app, id: prompt.id }],
        ["enable_prompt", { app, id: prompt.id }],
        ["import_prompt_from_file", { app }],
      ]),
    );
  });

  it("rejects invalid Prompt identifiers and malformed native payloads", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const ports = createTauriFeaturePorts();

    await expect(
      ports.prompts.getAll("claude-desktop" as PromptAppId),
    ).rejects.toThrow("application");
    await expect(ports.prompts.delete("claude", " prompt-a")).rejects.toThrow(
      "identifier",
    );
    await expect(
      ports.prompts.upsert("claude", {
        id: "prompt-a",
        name: "   ",
        content: "content",
        enabled: false,
      }),
    ).rejects.toThrow("name");
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValueOnce({
      "prompt-a": {
        id: "different-id",
        name: "Prompt A",
        content: "content",
        enabled: false,
      },
    });
    await expect(ports.prompts.getAll("claude")).rejects.toThrow();

    invoke.mockResolvedValueOnce({
      "prompt-a": {
        id: "prompt-a",
        name: "Prompt A",
        content: "content",
        enabled: false,
        updatedAt: "yesterday",
      },
    });
    await expect(ports.prompts.getAll("claude")).rejects.toThrow("unavailable");

    invoke.mockResolvedValueOnce(42);
    await expect(ports.prompts.getCurrentFileContent("claude")).rejects.toThrow(
      "live file",
    );

    invoke.mockResolvedValueOnce("");
    await expect(ports.prompts.importFromFile("claude")).rejects.toThrow(
      "Imported",
    );
  });

  it("maps the four Memory documents and daily resources to exact existing commands", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    invoke.mockImplementation(async (command: string) => {
      if (command === "read_workspace_file") return null;
      if (command === "get_hermes_memory") return "Hermes memory";
      if (command === "get_hermes_memory_limits") {
        return {
          memory: 2200,
          user: 1375,
          memoryEnabled: true,
          userEnabled: false,
        };
      }
      if (command === "list_daily_memory_files") {
        return [
          {
            filename: "2026-08-14.md",
            date: "2026-08-14",
            sizeBytes: 128,
            modifiedAt: 1_700_000_000,
            preview: "Daily preview",
          },
        ];
      }
      if (command === "read_daily_memory_file") return "Daily content";
      if (command === "search_daily_memory_files") {
        return [
          {
            filename: "2026-08-14.md",
            date: "2026-08-14",
            sizeBytes: 128,
            modifiedAt: 1_700_000_000,
            snippet: "Daily result",
            matchCount: 1,
          },
        ];
      }
      if (command === "open_workspace_directory") return true;
      return undefined;
    });

    const ports = createTauriFeaturePorts();
    await expect(
      ports.memory.readDocument("openclaw-memory"),
    ).resolves.toBeNull();
    await expect(
      ports.memory.readDocument("openclaw-user"),
    ).resolves.toBeNull();
    await expect(ports.memory.readDocument("hermes-memory")).resolves.toBe(
      "Hermes memory",
    );
    await expect(ports.memory.readDocument("hermes-user")).resolves.toBe(
      "Hermes memory",
    );
    await ports.memory.writeDocument("openclaw-memory", "OpenClaw M");
    await ports.memory.writeDocument("openclaw-user", "OpenClaw U");
    await ports.memory.writeDocument("hermes-memory", "Hermes M");
    await ports.memory.writeDocument("hermes-user", "Hermes U");
    await expect(ports.memory.getHermesLimits()).resolves.toEqual({
      memory: 2200,
      user: 1375,
      memoryEnabled: true,
      userEnabled: false,
    });
    await ports.memory.setHermesEnabled("memory", false);
    await ports.memory.setHermesEnabled("user", true);
    await expect(ports.memory.listDailyFiles()).resolves.toHaveLength(1);
    await expect(ports.memory.readDailyFile("2026-08-14.md")).resolves.toBe(
      "Daily content",
    );
    await ports.memory.writeDailyFile("2026-08-14.md", "Daily update");
    await ports.memory.deleteDailyFile("2026-08-14.md");
    await expect(ports.memory.searchDailyFiles("Daily")).resolves.toHaveLength(
      1,
    );
    await ports.memory.openOpenClawDirectory("workspace");
    await ports.memory.openOpenClawDirectory("memory");

    expect(invoke.mock.calls).toEqual([
      ["read_workspace_file", { filename: "MEMORY.md" }],
      ["read_workspace_file", { filename: "USER.md" }],
      ["get_hermes_memory", { kind: "memory" }],
      ["get_hermes_memory", { kind: "user" }],
      [
        "write_workspace_file",
        { filename: "MEMORY.md", content: "OpenClaw M" },
      ],
      ["write_workspace_file", { filename: "USER.md", content: "OpenClaw U" }],
      ["set_hermes_memory", { kind: "memory", content: "Hermes M" }],
      ["set_hermes_memory", { kind: "user", content: "Hermes U" }],
      ["get_hermes_memory_limits"],
      ["set_hermes_memory_enabled", { kind: "memory", enabled: false }],
      ["set_hermes_memory_enabled", { kind: "user", enabled: true }],
      ["list_daily_memory_files"],
      ["read_daily_memory_file", { filename: "2026-08-14.md" }],
      [
        "write_daily_memory_file",
        { filename: "2026-08-14.md", content: "Daily update" },
      ],
      ["delete_daily_memory_file", { filename: "2026-08-14.md" }],
      ["search_daily_memory_files", { query: "Daily" }],
      ["open_workspace_directory", { subdir: "workspace" }],
      ["open_workspace_directory", { subdir: "memory" }],
    ]);
  });

  it("rejects invalid Memory resources, dates, and malformed native payloads", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const ports = createTauriFeaturePorts();

    await expect(
      ports.memory.readDocument("codex-memory" as MemoryDocumentId),
    ).rejects.toThrow("document");
    await expect(
      ports.memory.setHermesEnabled("profile" as HermesMemoryKind, true),
    ).rejects.toThrow("kind");
    for (const filename of [
      "../MEMORY.md",
      "2026-2-03.md",
      "2026-02-30.md",
      "0000-01-01.md",
    ]) {
      await expect(ports.memory.readDailyFile(filename)).rejects.toThrow(
        "filename",
      );
      await expect(
        ports.memory.writeDailyFile(filename, "content"),
      ).rejects.toThrow("filename");
      await expect(ports.memory.deleteDailyFile(filename)).rejects.toThrow(
        "filename",
      );
    }
    expect(invoke).not.toHaveBeenCalled();

    invoke.mockResolvedValueOnce({
      memory: 2200,
      user: -1,
      memoryEnabled: true,
      userEnabled: true,
    });
    await expect(ports.memory.getHermesLimits()).rejects.toThrow("limits");

    invoke.mockResolvedValueOnce([
      {
        filename: "2026-02-30.md",
        date: "2026-02-30",
        sizeBytes: 1,
        modifiedAt: 1,
        preview: "bad date",
      },
    ]);
    await expect(ports.memory.listDailyFiles()).rejects.toThrow("filename");

    invoke.mockResolvedValueOnce([
      {
        filename: "2026-08-14.md",
        date: "2026-08-13",
        sizeBytes: 1,
        modifiedAt: 1,
        snippet: "mismatch",
        matchCount: 1,
      },
    ]);
    await expect(ports.memory.searchDailyFiles("query")).rejects.toThrow(
      "search",
    );

    invoke.mockResolvedValueOnce(false);
    await expect(
      ports.memory.openOpenClawDirectory("workspace"),
    ).rejects.toThrow("could not be opened");
  });

  it("rejects non-http external URLs before invoking native code", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    const ports = createTauriFeaturePorts();
    await expect(ports.settings.openExternal("file:///secret")).rejects.toThrow(
      "HTTP",
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it("opens a validated HTTP(S) URL through the exact native command", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/v2/shared/platform/tauri/features"
    );
    invoke.mockResolvedValue(undefined);
    const ports = createTauriFeaturePorts();
    await ports.settings.openExternal("https://qoder.com.cn/qoderwork");
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("open_external", {
      url: "https://qoder.com.cn/qoderwork",
    });
  });
});
