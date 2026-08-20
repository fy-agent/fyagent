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

    const delay = async (milliseconds = 0) => {
      if (milliseconds <= 0) return;
      await new Promise<void>((resolve) => {
        window.setTimeout(resolve, milliseconds);
      });
    };

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
            return { providers: [], revision: null };
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
              path: ".workbuddy/models.json",
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
            };
          }
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
