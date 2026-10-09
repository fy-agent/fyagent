// CC Switch v4.0.4 mode/stack commands; only the renderer port boundary differs.
import { invoke } from "@tauri-apps/api/core";
import * as z from "zod/mini";
import type { ProvidersPort } from "../../../features/ports";

const record = z.record(z.string(), z.unknown());
const provider = z.looseObject({
  id: z.string(),
  name: z.string(),
  settingsConfig: record,
  category: z.optional(
    z.enum([
      "official",
      "cn_official",
      "cloud_provider",
      "aggregator",
      "third_party",
      "custom",
      "omo",
      "omo-slim",
    ]),
  ),
  meta: z.optional(
    z.looseObject({
      stackModels: z.optional(
        z.array(
          z.object({
            model: z.string(),
            displayName: z.optional(z.string()),
            oneM: z.optional(z.boolean()),
          }),
        ),
      ),
      providerType: z.optional(z.string()),
      apiFormat: z.optional(
        z.enum([
          "anthropic",
          "openai_chat",
          "openai_responses",
          "gemini_native",
        ]),
      ),
      authBinding: z.optional(
        z.object({
          source: z.enum(["provider_config", "managed_account"]),
          authProvider: z.optional(z.string()),
          accountId: z.optional(z.string()),
        }),
      ),
    }),
  ),
});
const mode = z.object({
  mode: z.enum(["direct", "route", "stack"]),
  attached: z.boolean(),
  routeProviderId: z.nullable(z.string()),
  directProviderId: z.nullable(z.string()),
});
const stack = z.object({
  active: z.boolean(),
  members: z.array(
    z.object({
      providerId: z.string(),
      modelIds: z.array(z.string()),
      route: z.boolean(),
    }),
  ),
  notice: z.optional(
    z.enum([
      "routeOwnsCatalog",
      "officialModelsBundled",
      "officialModelsUnavailable",
    ]),
  ),
  staleClients: z.optional(
    z.object({ daemon: z.boolean(), others: z.boolean() }),
  ),
});

export const providerAggregationPorts: Pick<
  ProvidersPort,
  | "getAll"
  | "add"
  | "update"
  | "switch"
  | "getMode"
  | "setMode"
  | "setRoute"
  | "getStack"
  | "setStackMember"
  | "adoptCodexCatalog"
  | "restartCodexDaemon"
> = {
  getAll: async (app) =>
    z.parse(
      z.record(z.string(), provider),
      await invoke<unknown>("get_providers", { app }),
    ),
  add: (app, provider) =>
    invoke("add_provider", { app, provider, addToLive: false }),
  update: (app, provider) => invoke("update_provider", { app, provider }),
  switch: (app, id) => invoke("switch_provider", { app, id }),
  getMode: async (appType) =>
    z.parse(mode, await invoke<unknown>("get_app_mode", { appType })),
  setMode: async (appType, enabled, stack, route) => {
    await invoke("set_proxy_takeover_for_app", {
      appType,
      enabled,
      stack,
      route,
    });
    await invoke("update_tray_menu").catch(() => undefined);
  },
  setRoute: (appType, providerId) =>
    invoke("set_proxy_route", { appType, providerId }),
  getStack: async (appType) =>
    z.parse(stack, await invoke<unknown>("get_proxy_stack", { appType })),
  setStackMember: (appType, providerId, enabled) =>
    invoke("set_proxy_stack_member", { appType, providerId, enabled }),
  adoptCodexCatalog: () => invoke("adopt_codex_stack_catalog"),
  restartCodexDaemon: async () =>
    z.parse(
      z.enum(["restarted", "notRunning"]),
      await invoke<unknown>("restart_codex_app_server_daemon"),
    ),
};
