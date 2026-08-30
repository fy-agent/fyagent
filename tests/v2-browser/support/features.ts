import type { Page } from "@playwright/test";

export interface FeatureFixtureCall {
  command: string;
  payload: Record<string, unknown>;
}

export interface RichFeatureFixtureOptions {
  catalogFailure?: boolean;
  observationFailure?: "workbuddy" | "codex" | "claude";
  openExternalFailure?: boolean;
  existingQuickSetup?: "codex" | "claude";
  providerMutation?: "success" | "save_failure" | "switch_failure";
  providerWriteDelayMs?: number;
  workBuddySave?:
    | "saved"
    | "overwrite_then_saved"
    | "concurrent_modification"
    | "failure";
  workBuddyWriteDelayMs?: number;
}

declare global {
  interface Window {
    __FYAGENT_FEATURE_FIXTURE__: {
      calls: FeatureFixtureCall[];
    };
    __TAURI_INTERNALS__: {
      metadata: {
        currentWindow: { label: string };
        currentWebview: { label: string; windowLabel: string };
      };
      transformCallback: (callback: (event: unknown) => void) => number;
      invoke: (
        command: string,
        payload?: Record<string, unknown>,
      ) => Promise<unknown>;
    };
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: (event: string, eventId: number) => void;
    };
  }
}

export async function installRichTauriFeatureFixture(
  page: Page,
  options: RichFeatureFixtureOptions = {},
): Promise<void> {
  await page.addInitScript((fixtureOptions: RichFeatureFixtureOptions) => {
    const mcpAssignments = (enabled: string[]) =>
      Object.fromEntries(
        [
          "qoderwork",
          "trae-work",
          "workbuddy",
          "grokbuild",
          "codex",
          "claude",
          "opencode",
        ].map((id) => [id, enabled.includes(id)]),
      );
    const skillAssignments = (enabled: string[]): Record<string, boolean> =>
      mcpAssignments(enabled);
    const skills = [
      {
        id: "fixture-review",
        name: "Review Companion",
        description: "Deterministic browser acceptance fixture",
        directory: "review-companion",
        repoOwner: "fyagent-fixtures",
        repoName: "skills",
        repoBranch: "main",
        readmeUrl: "https://example.test/review-companion",
        apps: skillAssignments(["claude", "opencode"]),
        installedAt: 1_700_000_000,
        contentHash: "fixture-local-hash",
        updatedAt: 1_700_000_100,
      },
      {
        id: "fixture-notes",
        name: "Release Notes",
        description: "Second populated list item",
        directory: "release-notes",
        apps: skillAssignments(["codex"]),
        installedAt: 1_700_000_200,
        updatedAt: 1_700_000_300,
      },
    ];
    const mcpServers = {
      "fixture-context": {
        id: "fixture-context",
        name: "Fixture Context Server",
        description: "Populated stdio MCP fixture",
        tags: ["fixture", "browser"],
        source: "acceptance",
        server: {
          type: "stdio",
          command: "fixture-mcp",
          args: ["--safe-mode"],
          env: {
            FIXTURE_TOKEN: "synthetic-secret-never-render",
          },
          fixtureExtension: {
            retained: true,
          },
        },
        apps: mcpAssignments(["claude", "codex"]),
      },
      "fixture-http": {
        id: "fixture-http",
        name: "Fixture HTTP Server",
        description: "Second populated MCP item",
        server: {
          type: "http",
          url: "https://example.test/mcp",
          headers: {
            Authorization: "Bearer synthetic-header-never-render",
          },
        },
        apps: mcpAssignments(["workbuddy"]),
      },
    };
    const capabilityIds = [
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
    const catalogCapabilities = (id: string) =>
      capabilityIds.map((capabilityId) => ({
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
      }));
    const catalog = {
      contractVersion: 4,
      reviewedAt: "2026-08-20",
      agents: [
        {
          id: "qoderwork",
          variantId: "qoderwork-cn",
          displayName: "QoderWork CN",
          description: "Qoder 家族的桌面工作助手；当前仅提供官方入口。",
          officialLinks: [
            {
              id: "product",
              label: "打开 QoderWork 官方页面",
              url: "https://qoder.com.cn/qoderwork",
            },
          ],
          capabilities: catalogCapabilities("qoderwork"),
        },
        {
          id: "trae-work",
          variantId: "trae-work-cn",
          displayName: "TRAE Work CN",
          description:
            "支持 Skills 同步、模型配置与 MCP 直接分配；不支持 Hooks。",
          officialLinks: [
            {
              id: "product",
              label: "打开 TRAE Work CN 官方页面",
              url: "https://www.trae.cn/sem-work",
            },
          ],
          capabilities: catalogCapabilities("trae-work"),
        },
        {
          id: "workbuddy",
          variantId: "workbuddy",
          displayName: "WorkBuddy",
          description:
            "支持 Skills 同步、模型配置与 MCP 直接分配；不支持 Hooks。",
          officialLinks: [
            {
              id: "product",
              label: "打开 WorkBuddy 官方页面",
              url: "https://www.workbuddy.cn/",
            },
          ],
          capabilities: catalogCapabilities("workbuddy"),
        },
        {
          id: "grokbuild",
          variantId: "grokbuild",
          displayName: "Grok Build",
          description:
            "支持 Skills 同步、模型配置与 MCP 直接分配。本机识别和启动暂无法确认。",
          officialLinks: [
            {
              id: "product",
              label: "打开 Grok Build 官方页面",
              url: "https://x.ai/grok",
            },
          ],
          capabilities: catalogCapabilities("grokbuild"),
        },
        {
          id: "codex",
          variantId: "codex",
          displayName: "Codex",
          description: "支持桌面安装、Skills、模型配置与 MCP；不支持 Hooks。",
          officialLinks: [],
          capabilities: catalogCapabilities("codex"),
        },
        {
          id: "claude-code",
          variantId: "claude-code",
          displayName: "Claude Code",
          description: "支持 Skills、模型配置与 MCP；不支持 Hooks。",
          officialLinks: [
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
          ],
          capabilities: catalogCapabilities("claude-code"),
        },
        {
          id: "opencode",
          variantId: "opencode",
          displayName: "OpenCode",
          description: "支持 Skills、模型配置与 MCP；不支持 Hooks。",
          officialLinks: [
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
          ],
          capabilities: catalogCapabilities("opencode"),
        },
      ],
    };
    const quickSetupIds = {
      grokbuild: "fyagent-v2-quick-setup-grokbuild",
      codex: "fyagent-v2-quick-setup-codex",
      claude: "fyagent-v2-quick-setup-claude",
    } as const;
    const providers: Record<string, Record<string, Record<string, unknown>>> = {
      codex: {
        "fixture-codex-current": {
          id: "fixture-codex-current",
          name: "Fixture Codex Current",
        },
      },
      claude: {
        "fixture-claude-current": {
          id: "fixture-claude-current",
          name: "Fixture Claude Current",
        },
      },
    };
    const currentProviderIds: Record<string, string> = {
      codex: "fixture-codex-current",
      claude: "fixture-claude-current",
    };
    if (fixtureOptions.existingQuickSetup) {
      const app = fixtureOptions.existingQuickSetup;
      const id = quickSetupIds[app];
      providers[app][id] = {
        id,
        name: `Existing ${app} quick setup`,
      };
    }
    let workBuddyRevision = "fixture-revision-1";
    let workBuddyModelIds = ["existing-model"];
    let workBuddySaveAttempts = 0;
    let workBuddyPlan: Record<string, unknown> | null = null;
    let workBuddySaveRequest: Record<string, unknown> | null = null;
    const installerReleaseId = `v1:${"a".repeat(64)}`;
    const installerRemote = {
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
    const installerLocal = {
      state: "not_installed",
      platform: "windows",
      architecture: "x86_64",
    };
    let installerSequence = 0;
    let installerJob: Record<string, unknown> | null = null;
    let qoderHooksRevision = "fixture-qoder-hooks-revision-1";
    let qoderHookGroups: Array<Record<string, unknown>> = [];
    const traeRequestId = "123e4567-e89b-42d3-a456-426614174000";
    const makeInstallerJob = (
      stage: "checking" | "cancelled",
    ): Record<string, unknown> => ({
      jobId: "fixture-job-001",
      sequence: ++installerSequence,
      stage,
      release: structuredClone(installerRemote),
      startedAt: "2026-08-14T05:00:01Z",
      updatedAt: "2026-08-14T05:00:02Z",
      progress: null,
      cancellable: stage === "checking",
      result: null,
      error: null,
    });
    const calls: FeatureFixtureCall[] = [];
    let nextCallbackId = 1;
    const authAccountStates = new Map<string, "logged_in" | "logged_out">([
      ["claude-code", "logged_out"],
    ]);
    const authProviders = new Map<
      string,
      Array<{ providerId: string; label: string }>
    >([
      [
        "opencode",
        [
          {
            providerId: `p1:${"a".repeat(32)}`,
            label: "OpenAI",
          },
        ],
      ],
    ]);
    const authSessions = new Map<
      string,
      {
        snapshot: Record<string, unknown>;
        polls: number;
      }
    >();
    const authObservation = (agentId: string): Record<string, unknown> => {
      const base = {
        contractVersion: 1,
        agentId,
        checkedAt: "2026-08-30T00:00:00Z",
      };
      if (agentId === "claude-code") {
        return {
          kind: "account",
          ...base,
          ownership: "agent_owned",
          authority: "verified",
          state: authAccountStates.get(agentId) ?? "logged_out",
          allowedIntents: ["login", "logout"],
          reasonCodes: [],
        };
      }
      if (agentId === "opencode") {
        const providers = authProviders.get(agentId) ?? [];
        return {
          kind: "provider_connections",
          ...base,
          ownership: "provider_owned",
          authority: "verified",
          state: providers.length === 0 ? "empty" : "configured",
          providers: structuredClone(providers),
          allowedIntents: ["connect_provider", "logout"],
          reasonCodes: [],
        };
      }
      if (agentId === "codex") {
        return {
          kind: "fyagent_managed",
          ...base,
          ownership: "fyagent_managed",
          authority: "verified",
          destination: "auth_center",
          allowedIntents: [],
          reasonCodes: ["managed_by_auth_center"],
        };
      }
      if (
        ["grokbuild", "qoderwork", "trae-work", "workbuddy"].includes(agentId)
      ) {
        return {
          kind: "handoff_only",
          ...base,
          ownership: "agent_owned",
          authority: "unverified",
          allowedIntents: ["login", "logout"],
          reasonCodes: ["handoff_only"],
        };
      }
      return {
        kind: "unavailable",
        ...base,
        ownership: "unavailable",
        authority: "unavailable",
        allowedIntents: [],
        reasonCodes: ["auth_observer_unavailable"],
      };
    };

    const delay = async (milliseconds = 0) => {
      if (milliseconds <= 0) return;
      await new Promise<void>((resolve) => {
        window.setTimeout(resolve, milliseconds);
      });
    };
    const digest = (value: string) => value.repeat(64);
    const changePlanNow = () => Math.floor(Date.now() / 1000);
    const upsertAdapter = {
      adapterId: "codex_provider_upsert_and_switch",
      adapterVersion: "1",
      operationType: "codex_provider_upsert_and_switch",
      phases: ["precheck", "snapshot", "managed_write", "readback", "finalize"],
      readSet: [
        "provider_db_current",
        "device_current",
        "target_definition",
        "codex_live_projection",
      ],
      writeSet: [
        "provider_db_current",
        "device_current",
        "codex_live_projection",
      ],
      idempotencyScope: "plan",
      cancelMode: "before_managed_write",
      compensationMode: "writer_owned_rollback",
      faultPoints: [
        "before_managed_write",
        "after_managed_write_before_record",
      ],
    };
    let upsertPlan: Record<string, unknown> | null = null;
    let changeJob: Record<string, unknown> | null = null;
    const makeUpsertPlan = (name: string) => {
      const createdAt = changePlanNow();
      return {
        planId: "plan-codex-upsert",
        operation: "codex_provider_upsert_and_switch",
        targetProviderId: "fyagent-v2-quick-setup-codex",
        targetProviderName: name,
        planDigest: digest("a"),
        baselineDigest: digest("b"),
        dbBaselineProviderId: null,
        deviceBaselineProviderId: currentProviderIds.codex ?? null,
        secretCapability: "no_new_credential_material",
        createdAt,
        expiresAt: createdAt + 900,
        status: "ready",
        adapter: upsertAdapter,
        currentProviderCode: "current_mixed",
        targetProviderCode: providers.codex?.["fyagent-v2-quick-setup-codex"]
          ? "quick_setup_update"
          : "quick_setup_create",
        restartExpectation: "recommended",
        risks: [
          { code: "local_configuration_write", severity: "notice" },
          { code: "save_provider_then_set_current", severity: "notice" },
        ],
        evidenceNote: "usage_not_observed",
      };
    };
    const workBuddyAdapter = {
      adapterId: "workbuddy_models_save",
      adapterVersion: "1",
      operationType: "workbuddy_models_save",
      phases: ["precheck", "snapshot", "managed_write", "readback", "finalize"],
      readSet: ["work_buddy_models_config", "work_buddy_backup"],
      writeSet: ["work_buddy_models_config", "work_buddy_backup"],
      idempotencyScope: "plan",
      cancelMode: "before_managed_write",
      compensationMode: "writer_owned_rollback",
      faultPoints: [
        "before_managed_write",
        "after_managed_write_before_record",
      ],
    };
    const makeWorkBuddyPlan = (overwrite: boolean) => {
      const createdAt = changePlanNow();
      return {
        planId: "plan-workbuddy-save",
        operation: "workbuddy_models_save",
        targetProviderId: "fyagent-v2-workbuddy-models",
        targetProviderName: "https://workbuddy.example.test/v1",
        planDigest: digest("c"),
        baselineDigest: digest("d"),
        dbBaselineProviderId: null,
        deviceBaselineProviderId: null,
        secretCapability: "no_new_credential_material",
        createdAt,
        expiresAt: createdAt + 900,
        status: "ready",
        adapter: workBuddyAdapter,
        currentProviderCode: "object_root",
        targetProviderCode: "object_root",
        restartExpectation: "not_required",
        risks: overwrite
          ? [
              { code: "local_configuration_write", severity: "notice" },
              {
                code: "existing_model_ids_will_be_updated",
                severity: "warning",
              },
            ]
          : [{ code: "local_configuration_write", severity: "notice" }],
        evidenceNote: "usage_not_observed",
      };
    };
    const makeWorkBuddyJob = (failed: boolean) => ({
      jobId: "job-workbuddy-save",
      executionId: "job-workbuddy-save",
      planId: "plan-workbuddy-save",
      idempotencyKey: "plan-workbuddy-save",
      targetProviderId: "fyagent-v2-workbuddy-models",
      revision: 5,
      eventSeq: 5,
      status: failed ? "failed" : "succeeded",
      resultCode: failed ? "writer_failed_baseline_restored" : "applied",
      adapterErrorCode: failed ? "writer_failed" : null,
      steps: failed
        ? [
            { kind: "precheck", status: "succeeded", code: "ok" },
            { kind: "snapshot", status: "succeeded", code: "ok" },
            { kind: "managed_write", status: "compensated", code: "ok" },
            { kind: "readback", status: "succeeded", code: "ok" },
            { kind: "finalize", status: "succeeded", code: "ok" },
          ]
        : [
            { kind: "precheck", status: "succeeded", code: "ok" },
            { kind: "snapshot", status: "succeeded", code: "ok" },
            { kind: "managed_write", status: "succeeded", code: "ok" },
            { kind: "readback", status: "succeeded", code: "ok" },
            { kind: "finalize", status: "succeeded", code: "ok" },
          ],
      resources: [
        { kind: "work_buddy_models_config", status: "matched", code: "ok" },
        { kind: "work_buddy_backup", status: "matched", code: "ok" },
      ],
      partialResult: failed
        ? {
            succeededSteps: ["precheck", "snapshot", "readback", "finalize"],
            compensatedSteps: ["managed_write"],
            unverifiedSteps: [],
            remainingEffects: [],
            manualActions: [],
          }
        : {
            succeededSteps: [
              "precheck",
              "snapshot",
              "managed_write",
              "readback",
              "finalize",
            ],
            compensatedSteps: [],
            unverifiedSteps: [],
            remainingEffects: [],
            manualActions: [],
          },
      events: [
        { sequence: 1, phase: "precheck", reasonCode: "ok", createdAt: 1 },
        { sequence: 2, phase: "snapshot", reasonCode: "ok", createdAt: 2 },
        {
          sequence: 3,
          phase: "managed_write",
          reasonCode: failed ? "writer_owned_rollback_confirmed" : "ok",
          createdAt: 3,
        },
        { sequence: 4, phase: "readback", reasonCode: "ok", createdAt: 4 },
        { sequence: 5, phase: "finalize", reasonCode: "ok", createdAt: 5 },
      ],
      restartRequirement: "not_required",
      usageEvidence: "not_observed",
      recoveryState: failed ? "succeeded" : "not_needed",
      diagnosticCode: null,
      liveConfigChanged: false,
      createdAt: 1,
      updatedAt: 5,
    });
    const makeTerminalJob = (failed: boolean) => ({
      jobId: "job-codex-upsert",
      executionId: "job-codex-upsert",
      planId: "plan-codex-upsert",
      idempotencyKey: "plan-codex-upsert",
      targetProviderId: "fyagent-v2-quick-setup-codex",
      revision: 5,
      eventSeq: 5,
      status: failed ? "failed" : "succeeded",
      resultCode: failed ? "writer_failed_baseline_restored" : "applied",
      adapterErrorCode: failed ? "writer_failed" : null,
      steps: failed
        ? [
            { kind: "precheck", status: "succeeded", code: "ok" },
            { kind: "snapshot", status: "succeeded", code: "ok" },
            { kind: "managed_write", status: "failed", code: "writer_failed" },
            { kind: "readback", status: "skipped", code: "skipped" },
            { kind: "finalize", status: "skipped", code: "skipped" },
          ]
        : [
            { kind: "precheck", status: "succeeded", code: "ok" },
            { kind: "snapshot", status: "succeeded", code: "ok" },
            { kind: "managed_write", status: "succeeded", code: "ok" },
            { kind: "readback", status: "succeeded", code: "ok" },
            { kind: "finalize", status: "succeeded", code: "ok" },
          ],
      resources: [
        { kind: "provider_db_current", status: "matched", code: "ok" },
        { kind: "device_current", status: "matched", code: "ok" },
        { kind: "target_definition", status: "matched", code: "ok" },
        { kind: "codex_live_projection", status: "matched", code: "ok" },
      ],
      partialResult: failed
        ? {
            succeededSteps: ["precheck", "snapshot"],
            compensatedSteps: ["managed_write"],
            unverifiedSteps: [],
            remainingEffects: [],
            manualActions: [],
          }
        : {
            succeededSteps: [
              "precheck",
              "snapshot",
              "managed_write",
              "readback",
              "finalize",
            ],
            compensatedSteps: [],
            unverifiedSteps: [],
            remainingEffects: [],
            manualActions: [],
          },
      events: [
        { sequence: 1, phase: "precheck", reasonCode: "ok", createdAt: 1 },
        { sequence: 2, phase: "snapshot", reasonCode: "ok", createdAt: 2 },
        {
          sequence: 3,
          phase: "managed_write",
          reasonCode: failed ? "writer_failed" : "ok",
          createdAt: 3,
        },
        {
          sequence: 4,
          phase: "readback",
          reasonCode: failed ? "skipped" : "ok",
          createdAt: 4,
        },
        {
          sequence: 5,
          phase: "finalize",
          reasonCode: failed ? "skipped" : "ok",
          createdAt: 5,
        },
      ],
      restartRequirement: failed ? "unknown" : "recommended",
      usageEvidence: "not_observed",
      recoveryState: failed ? "succeeded" : "not_needed",
      diagnosticCode: null,
      liveConfigChanged: !failed,
      createdAt: 1,
      updatedAt: 5,
    });

    window.__FYAGENT_FEATURE_FIXTURE__ = { calls };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => undefined,
    };
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main", windowLabel: "main" },
      },
      transformCallback: () => nextCallbackId++,
      invoke: async (
        command: string,
        payload: Record<string, unknown> = {},
      ) => {
        calls.push({
          command,
          payload: structuredClone(payload),
        });
        switch (command) {
          case "get_agent_catalog":
            if (fixtureOptions.catalogFailure) {
              throw new Error("fixture catalog unavailable");
            }
            return structuredClone(catalog);
          case "get_external_agent_status":
            return {
              agentId: payload.agentId,
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
          case "launch_external_agent":
            return {
              agentId: payload.agentId,
              destination: payload.destination,
              state: "unverified",
              reasonCode: "trusted_runtime_identity_unavailable",
            };
          case "get_qoderwork_hooks":
            return {
              revision: qoderHooksRevision,
              exists: qoderHookGroups.length > 0,
              groups: structuredClone(qoderHookGroups),
              restartRequired: true,
              supportedStructure: true,
            };
          case "save_qoderwork_hooks": {
            const request = payload.request as Record<string, unknown>;
            qoderHookGroups = structuredClone(
              (request.groups as Array<Record<string, unknown>>) ?? [],
            );
            qoderHooksRevision = "fixture-qoder-hooks-revision-2";
            return {
              state: "saved",
              snapshot: {
                revision: qoderHooksRevision,
                exists: true,
                groups: structuredClone(qoderHookGroups),
                restartRequired: true,
                supportedStructure: true,
              },
            };
          }
          case "validate_external_mcp_config": {
            const config = payload.config as {
              mcpServers?: Record<string, Record<string, unknown>>;
            };
            const findings = Object.entries(config.mcpServers ?? {}).map(
              ([serverId, server]) => ({
                serverId,
                transport: "url" in server ? "http" : "stdio",
                reasonCodes: ["TRAE_MCP_SERVER_VALID"],
                executableAvailable: "url" in server ? null : true,
                hasSecrets: "env" in server || "headers" in server,
              }),
            );
            const redactedServers = Object.fromEntries(
              Object.entries(config.mcpServers ?? {}).map(
                ([serverId, server]) => [
                  serverId,
                  {
                    ...server,
                    ...(server.env ? { env: { REDACTED: "<redacted>" } } : {}),
                    ...(server.headers
                      ? { headers: { REDACTED: "<redacted>" } }
                      : {}),
                  },
                ],
              ),
            );
            return {
              agentId: payload.agentId,
              valid: true,
              findings,
              redactedTemplate: { mcpServers: redactedServers },
            };
          }
          case "validate_traework_model_config":
            return {
              requestId: traeRequestId,
              state: "valid",
              reasonCode: "TRAE_MODEL_CONFIG_VALID",
              durationBucket: "lt_1s",
              statusClass: null,
            };
          case "test_traework_model_endpoint":
            return {
              requestId: payload.requestId,
              state: "reachable",
              reasonCode: "TRAE_ENDPOINT_REACHABLE",
              durationBucket: "1s_to_3s",
              statusClass: "2xx",
            };
          case "cancel_traework_model_endpoint":
            return { requestId: payload.requestId, cancelled: true };
          case "get_traework_model_ids":
            return {
              modelIds: ["fixture-model"],
              revision: "fixture-trae-revision",
              truncated: false,
            };
          case "get_opencode_model_snapshot":
            return {
              providers: [],
              revision: null,
              path: "~/.config/opencode/opencode.json",
              backupPath: "~/.config/opencode/opencode.json.backup",
              exists: true,
            };
          case "fetch_opencode_provider_models":
            return {
              models: [{ id: "fixture-opencode-model" }],
              truncated: false,
            };
          case "save_opencode_models":
            return {
              state: "saved",
              revision: "fixture-opencode-revision",
              modelCount: 1,
              createdEntries: 1,
              updatedEntries: 0,
            };
          case "codex_desktop_get_local_status":
            return structuredClone(installerLocal);
          case "codex_desktop_check_latest":
            return structuredClone(installerRemote);
          case "codex_desktop_get_job":
            return structuredClone(installerJob);
          case "codex_desktop_start_install":
            installerJob = makeInstallerJob("checking");
            return structuredClone(installerJob);
          case "codex_desktop_cancel_install":
            installerJob = makeInstallerJob("cancelled");
            return structuredClone(installerJob);
          case "codex_desktop_launch":
          case "codex_desktop_open_log_directory":
            return undefined;
          case "get_workbuddy_status":
            if (fixtureOptions.observationFailure === "workbuddy") {
              throw {
                code: "WORKBUDDY_CONFIG_READ_FAILED",
                messageKey: "workbuddy.error.configReadFailed",
                details: {},
              };
            }
            return {
              path: "~/.workbuddy/models.json",
              backupPath: "~/.workbuddy/models.json.backup",
              exists: true,
              modelCount: workBuddyModelIds.length,
              revision: workBuddyRevision,
              backupExists: true,
              format: "objectRoot",
            };
          case "get_workbuddy_model_ids":
            return {
              ids: structuredClone(workBuddyModelIds),
              revision: workBuddyRevision,
            };
          case "fetch_workbuddy_models":
            return {
              models: ["fixture-model-alpha", "fixture-model-beta"],
              truncated: false,
            };
          case "save_workbuddy_models": {
            await delay(fixtureOptions.workBuddyWriteDelayMs);
            workBuddySaveAttempts += 1;
            if (fixtureOptions.workBuddySave === "failure") {
              throw {
                code: "WORKBUDDY_CONFIG_WRITE_FAILED",
                messageKey: "workbuddy.error.configWriteFailed",
                details: {},
              };
            }
            if (fixtureOptions.workBuddySave === "concurrent_modification") {
              return { state: "concurrent_modification" };
            }
            const request = payload.request as
              | Record<string, unknown>
              | undefined;
            if (
              fixtureOptions.workBuddySave === "overwrite_then_saved" &&
              workBuddySaveAttempts === 1
            ) {
              return {
                state: "overwrite_confirmation_required",
                token: "fixture-opaque-overwrite-token",
                existingIds: ["existing-model"],
              };
            }
            if (
              fixtureOptions.workBuddySave === "overwrite_then_saved" &&
              request?.overwriteToken !== "fixture-opaque-overwrite-token"
            ) {
              throw {
                code: "WORKBUDDY_OVERWRITE_TOKEN_INVALID",
                messageKey: "workbuddy.error.overwriteTokenInvalid",
                details: {},
              };
            }
            workBuddyModelIds = [
              ...new Set(
                [
                  ...((request?.selectedModelIds as string[] | undefined) ??
                    []),
                  ...((request?.manualModelIds as string[] | undefined) ?? []),
                ].filter(Boolean),
              ),
            ];
            workBuddyRevision = `fixture-revision-${workBuddySaveAttempts + 1}`;
            return {
              state: "saved",
              revision: workBuddyRevision,
              modelCount: workBuddyModelIds.length,
              createdEntries: workBuddyModelIds.length,
              updatedEntries: 0,
            };
          }
          case "get_provider_summary": {
            const app = String(payload.app);
            if (fixtureOptions.observationFailure === app) {
              throw new Error(
                `fixture ${app} Provider observation unavailable`,
              );
            }
            return {
              providers: structuredClone(providers[app] ?? {}),
              currentId: currentProviderIds[app] ?? "",
              writeTargets: [
                {
                  path: `~/.config/${app}/config`,
                  backupPath: `~/.config/${app}/config.fyagent.backup`,
                  exists: true,
                },
              ],
            };
          }
          case "list_recoverable_change_jobs":
            return [];
          case "create_codex_provider_switch_plan": {
            const createdAt = changePlanNow();
            return {
              planId: "plan-codex-switch",
              operation: "codex_provider_switch",
              targetProviderId: String(payload.targetProviderId),
              targetProviderName: "Fixture Codex Switch",
              planDigest: digest("c"),
              baselineDigest: digest("d"),
              dbBaselineProviderId: currentProviderIds.codex ?? null,
              deviceBaselineProviderId: currentProviderIds.codex ?? null,
              secretCapability: "no_new_credential_material",
              createdAt,
              expiresAt: createdAt + 900,
              status: "ready",
              adapter: {
                ...upsertAdapter,
                adapterId: "codex_provider_switch",
                operationType: "codex_provider_switch",
              },
              currentProviderCode: "current_mixed",
              targetProviderCode: "existing_provider",
              restartExpectation: "recommended",
              risks: [
                { code: "local_configuration_write", severity: "notice" },
              ],
              evidenceNote: "usage_not_observed",
            };
          }
          case "create_codex_provider_upsert_plan": {
            await delay(fixtureOptions.providerWriteDelayMs);
            const request = payload.request as
              | Record<string, unknown>
              | undefined;
            upsertPlan = makeUpsertPlan(String(request?.name ?? "Codex"));
            changeJob = null;
            return structuredClone(upsertPlan);
          }
          case "create_workbuddy_save_plan": {
            const request = payload.request as
              | Record<string, unknown>
              | undefined;
            workBuddySaveRequest = structuredClone(request ?? {});
            workBuddyPlan = makeWorkBuddyPlan(
              fixtureOptions.workBuddySave === "overwrite_then_saved",
            );
            changeJob = null;
            return structuredClone(workBuddyPlan);
          }
          case "apply_change_plan": {
            if (workBuddyPlan && payload.planId === workBuddyPlan.planId) {
              if (payload.planDigest !== workBuddyPlan.planDigest) {
                return { kind: "rejected", errorCode: "stale" };
              }
              if (fixtureOptions.workBuddySave === "concurrent_modification") {
                return { kind: "rejected", errorCode: "stale" };
              }
              const failed = fixtureOptions.workBuddySave === "failure";
              if (!failed && workBuddySaveRequest) {
                workBuddyModelIds = [
                  ...new Set(
                    [
                      ...((workBuddySaveRequest.selectedModelIds as
                        | string[]
                        | undefined) ?? []),
                      ...((workBuddySaveRequest.manualModelIds as
                        | string[]
                        | undefined) ?? []),
                    ].filter(Boolean),
                  ),
                ];
                workBuddyRevision = "fixture-revision-applied";
              }
              workBuddyPlan = { ...workBuddyPlan, status: "consumed" };
              changeJob = makeWorkBuddyJob(failed);
              return {
                kind: "admitted",
                job: structuredClone(changeJob),
              };
            }
            if (
              !upsertPlan ||
              payload.planId !== upsertPlan.planId ||
              payload.planDigest !== upsertPlan.planDigest
            ) {
              return { kind: "rejected", errorCode: "stale" };
            }
            const failed = fixtureOptions.providerMutation === "switch_failure";
            if (!failed) {
              const providerId = "fyagent-v2-quick-setup-codex";
              providers.codex ??= {};
              providers.codex[providerId] = {
                id: providerId,
                name: String(upsertPlan.targetProviderName),
              };
              currentProviderIds.codex = providerId;
            }
            upsertPlan = { ...upsertPlan, status: "consumed" };
            changeJob = makeTerminalJob(failed);
            return {
              kind: "admitted",
              job: structuredClone(changeJob),
            };
          }
          case "get_change_job":
            if (!changeJob || payload.jobId !== changeJob.jobId) {
              throw new Error("fixture Change Job not found");
            }
            return structuredClone(changeJob);
          case "cancel_change_job":
            return {
              accepted: true,
              code: "accepted",
              jobId: String(payload.jobId),
            };
          case "apply_provider_quick_setup_with_result": {
            await delay(fixtureOptions.providerWriteDelayMs);
            if (fixtureOptions.providerMutation === "save_failure") {
              throw new Error("fixture Provider atomic apply rejected");
            }
            if (fixtureOptions.providerMutation === "switch_failure") {
              throw {
                code: "APPLY_FAILED_ROLLED_BACK",
                message: "fixture Provider atomic apply rolled back",
              };
            }
            const app = String(payload.app);
            const request = structuredClone(
              payload.request as Record<string, unknown>,
            );
            const providerId = `fyagent-v2-quick-setup-${app}`;
            providers[app] ??= {};
            providers[app][providerId] = {
              id: providerId,
              name: String(request.name),
            };
            currentProviderIds[app] = providerId;
            return {
              value: { warnings: [] },
              liveConfigChanged: app === "codex",
              app,
              warningCodes:
                app === "codex" ? ["CODEX_WEBSOCKET_NON_GPT_MODEL"] : [],
            };
          }
          case "open_external":
            if (fixtureOptions.openExternalFailure) {
              throw new Error("fixture external open rejected");
            }
            return undefined;
          case "get_installed_skills":
            return structuredClone(skills);
          case "get_mcp_servers":
            return structuredClone(mcpServers);
          case "toggle_skill_app": {
            const skill = skills.find((item) => item.id === payload.id);
            const app = String(payload.app);
            if (skill) skill.apps[app] = Boolean(payload.enabled);
            return Boolean(skill);
          }
          case "toggle_mcp_app": {
            const server =
              mcpServers[String(payload.serverId) as keyof typeof mcpServers];
            const app = String(payload.app);
            if (server) server.apps[app] = Boolean(payload.enabled);
            return undefined;
          }
          case "get_skill_backups":
          case "scan_unmanaged_skills":
          case "discover_available_skills":
          case "check_skill_updates":
          case "get_skill_repos":
            return [];
          case "discover_available_skills_page":
            return { skills: [], totalCount: 0 };
          case "search_skillhub":
            return {
              skills: [],
              totalCount: 0,
              query: payload.query ?? "",
              categories: [],
            };
          case "install_skillhub":
            return [];
          case "get_agent_install_readiness": {
            const agentId = String(payload.agentId);
            const cliAgent = ["grokbuild", "claude-code", "opencode"].includes(
              agentId,
            );
            return {
              contractVersion: 3,
              agentId,
              reviewedAt: "2026-08-29",
              installState: "installed",
              inventoryState: "single",
              requiresTargetSelection: false,
              updateState: "up_to_date",
              releaseId: null,
              localVersion: "1.0.0",
              remoteVersion: null,
              authOwnership:
                agentId === "codex" ? "fyagent_managed" : "agent_owned",
              authState: "unknown",
              sourceKind:
                agentId === "codex"
                  ? "codex_desktop"
                  : cliAgent
                    ? "cli_tooling"
                    : "managed_desktop",
              allowedActions: [],
              reasonCodes: ["auth_state_unknown"],
            };
          }
          case "get_agent_auth_observation":
            return structuredClone(authObservation(String(payload.agentId)));
          case "get_active_agent_auth_session": {
            const agentId = String(payload.agentId);
            const record = [...authSessions.values()].find(
              ({ snapshot }) =>
                snapshot.agentId === agentId &&
                ![
                  "verified",
                  "handoff_complete",
                  "failed",
                  "cancelled",
                  "timed_out",
                ].includes(String(snapshot.stage)),
            );
            return record ? structuredClone(record.snapshot) : null;
          }
          case "start_agent_auth_session": {
            const request = payload.request as {
              agentId: string;
              intent: "login" | "logout" | "connect_provider";
              providerId?: string;
            };
            const sessionId = crypto.randomUUID();
            if (request.agentId === "codex") {
              throw { reasonCode: "managed_by_auth_center" };
            }
            if (request.agentId === "opencode") {
              const providers = authProviders.get("opencode") ?? [];
              if (request.intent === "logout" && request.providerId) {
                authProviders.set(
                  "opencode",
                  providers.filter(
                    (provider) => provider.providerId !== request.providerId,
                  ),
                );
              } else if (request.intent === "connect_provider") {
                authProviders.set("opencode", [
                  ...providers,
                  {
                    providerId: `p1:${"b".repeat(32)}`,
                    label: "Anthropic",
                  },
                ]);
              }
              return {
                contractVersion: 1,
                sessionId,
                agentId: request.agentId,
                intent: request.intent,
                stage: "verified",
                canStopWaiting: false,
                outcome: "verified_provider_change",
                observation: authObservation(request.agentId),
                reasonCode: null,
              };
            }
            if (request.agentId !== "claude-code") {
              return {
                contractVersion: 1,
                sessionId,
                agentId: request.agentId,
                intent: request.intent,
                stage: "handoff_complete",
                canStopWaiting: false,
                outcome: "handoff_only",
                observation: authObservation(request.agentId),
                reasonCode: "handoff_only",
              };
            }
            const snapshot = {
              contractVersion: 1,
              sessionId,
              agentId: request.agentId,
              intent: request.intent,
              stage: "awaiting_user",
              canStopWaiting: true,
              outcome: null,
              observation: authObservation(request.agentId),
              reasonCode: null,
            };
            authSessions.set(sessionId, { snapshot, polls: 0 });
            return structuredClone(snapshot);
          }
          case "get_agent_auth_session": {
            const sessionId = String(payload.sessionId);
            const record = authSessions.get(sessionId);
            if (!record) throw { reasonCode: "operation_conflict" };
            record.polls += 1;
            if (record.polls >= 1) {
              const intent = String(record.snapshot.intent);
              authAccountStates.set(
                "claude-code",
                intent === "logout" ? "logged_out" : "logged_in",
              );
              record.snapshot = {
                ...record.snapshot,
                stage: "verified",
                canStopWaiting: false,
                outcome:
                  intent === "logout"
                    ? "verified_logged_out"
                    : "verified_logged_in",
                observation: authObservation("claude-code"),
              };
            }
            return structuredClone(record.snapshot);
          }
          case "stop_waiting_for_agent_auth": {
            const sessionId = String(payload.sessionId);
            const record = authSessions.get(sessionId);
            if (!record) throw { reasonCode: "operation_conflict" };
            record.snapshot = {
              ...record.snapshot,
              stage: "cancelled",
              canStopWaiting: false,
              outcome: "cancelled",
              reasonCode: "monitoring_stopped",
            };
            return structuredClone(record.snapshot);
          }
          case "get_settings":
            return { skillSyncMethod: "auto", skillStorageLocation: "fyagent" };
          case "plugin:event|listen":
            return payload.handler;
          case "plugin:event|emit":
          case "plugin:event|unlisten":
            return undefined;
          default:
            throw new Error(`Unexpected fixture command: ${command}`);
        }
      },
    };
  }, options);
}

export async function featureFixtureCalls(
  page: Page,
): Promise<FeatureFixtureCall[]> {
  return page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.calls);
}
