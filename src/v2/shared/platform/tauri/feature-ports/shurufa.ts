import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  ShurufaConfig,
  ShurufaEvent,
  ShurufaPort,
  ShurufaSnapshot,
} from "../../../features/ports";

const EVENT_NAME = "shurufa://event";

function parseConfig(value: unknown): ShurufaConfig {
  const record = asRecord(value);
  return {
    url: asString(record.url),
    model: asString(record.model),
    apiKey: asString(record.apiKey),
    maxSummaries: asNumber(record.maxSummaries, 8),
    timeoutSecs: asNumber(record.timeoutSecs, 60),
    configured: record.configured === true,
  };
}

function parseSnapshot(value: unknown): ShurufaSnapshot {
  const record = asRecord(value);
  return {
    prompt: asString(record.prompt),
    config: parseConfig(record.config),
    running: record.running === true,
    lastOutput: asString(record.lastOutput),
    lastError:
      typeof record.lastError === "string" ? record.lastError : null,
    shortcutLabel: asString(record.shortcutLabel) || "⌘M",
    dataDir: asString(record.dataDir),
  };
}

function parseEvent(value: unknown): ShurufaEvent | null {
  const record = asRecord(value);
  switch (record.type) {
    case "started":
      return { type: "started" };
    case "delta":
      return { type: "delta", text: asString(record.text) };
    case "finished":
      return { type: "finished", output: asString(record.output) };
    case "error":
      return { type: "error", message: asString(record.message) };
    default:
      return null;
  }
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object"
    ? (value as Record<string, unknown>)
    : {};
}

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function asNumber(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

export function createShurufaPort(): ShurufaPort {
  return {
    getSnapshot: async () => parseSnapshot(await invoke("shurufa_get_snapshot")),
    setPrompt: (text) => invoke("shurufa_set_prompt", { text }),
    saveConfig: async (config) =>
      parseConfig(
        await invoke("shurufa_save_config", {
          input: {
            url: config.url,
            model: config.model,
            apiKey: config.apiKey,
            maxSummaries: config.maxSummaries,
            timeoutSecs: config.timeoutSecs,
          },
        }),
      ),
    clearSession: () => invoke("shurufa_clear_session"),
    run: () => invoke("shurufa_run"),
    subscribe: async (onEvent) => {
      const unlisten = await listen<unknown>(EVENT_NAME, (event) => {
        const parsed = parseEvent(event.payload);
        if (parsed) onEvent(parsed);
      });
      return unlisten;
    },
  };
}
