import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  CompanionDeviceSettings,
  CompanionMapping,
  CompanionNetwork,
  CompanionProfile,
  CompanionRuntime,
  CompanionSnapshot,
  CompanionTarget,
  ShurufaConfig,
  ShurufaEvent,
  ShurufaPort,
  ShurufaSnapshot,
} from "../../../features/ports";
import {
  COMPANION_ASR_ADMISSIONS,
  COMPANION_INPUT_IDS,
  COMPANION_NETWORK_STATES,
  COMPANION_RUNTIME_STATES,
  DEFAULT_COMPANION_BAUD,
  DEFAULT_COMPANION_DEVICE_MODEL,
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
    lastError: typeof record.lastError === "string" ? record.lastError : null,
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

function parsePorts(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

function parseTarget(value: unknown): CompanionTarget | null {
  if (!value || typeof value !== "object") return null;
  const record = asRecord(value);
  return {
    processName: asString(record.processName),
    processPath: asString(record.processPath),
  };
}

function parseMapping(value: unknown): CompanionMapping | null {
  const record = asRecord(value);
  const input = asClosed(record.input, COMPANION_INPUT_IDS);
  if (!input) return null;
  return {
    input,
    displayName: asString(record.displayName),
    keys: parsePorts(record.keys),
  };
}

function parseProfile(value: unknown): CompanionProfile | null {
  if (value == null) return null;
  const record = asRecord(value);
  const serial = asRecord(record.serial);
  const mappings = Array.isArray(record.mappings)
    ? record.mappings
        .map(parseMapping)
        .filter((mapping): mapping is CompanionMapping => mapping !== null)
    : [];
  return {
    version: 1,
    revision: typeof record.revision === "string" ? record.revision : null,
    serial: {
      port: asString(serial.port),
      baud: asNumber(serial.baud, DEFAULT_COMPANION_BAUD),
    },
    target: parseTarget(record.target),
    mappings,
  };
}

function parseDeviceSettings(value: unknown): CompanionDeviceSettings {
  const record = asRecord(value);
  return {
    version: 1,
    ssid: asString(record.ssid),
    password: asString(record.password),
    apiKey: asString(record.apiKey),
    model: asString(record.model) || DEFAULT_COMPANION_DEVICE_MODEL,
  };
}

function parseNetwork(value: unknown): CompanionNetwork {
  const record = asRecord(value);
  return {
    state: asClosed(record.state, COMPANION_NETWORK_STATES) ?? "UNKNOWN",
    ssid: asString(record.ssid),
    ip: asString(record.ip),
    rssi: asNullableNumber(record.rssi),
    reason: asNullableString(record.reason),
    pingHost: asNullableString(record.pingHost),
    pingOk: asNullableBoolean(record.pingOk),
    pingMs: asNullableNumber(record.pingMs),
    pingLost: asNullableNumber(record.pingLost),
    pingSent: asNullableNumber(record.pingSent),
    lastLog: asNullableString(record.lastLog),
    beats: asNullableNumber(record.beats),
    recState: asNullableString(record.recState),
    recMs: asNullableNumber(record.recMs),
    recSamples: asNullableNumber(record.recSamples),
    recRms: asNullableNumber(record.recRms),
    recPeak: asNullableNumber(record.recPeak),
    recSilence: asNullableBoolean(record.recSilence),
    recReason: asNullableString(record.recReason),
    asrState: asNullableString(record.asrState),
    asrText: asNullableString(record.asrText),
    asrReason: asNullableString(record.asrReason),
  };
}

function parseRuntime(value: unknown): CompanionRuntime {
  const record = asRecord(value);
  return {
    state: asClosed(record.state, COMPANION_RUNTIME_STATES) ?? "STOPPED",
    liveEnabled: record.liveEnabled === true,
    lastEvent: asString(record.lastEvent),
    gapMissed: asNullableNumber(record.gapMissed),
    network: parseNetwork(record.network),
  };
}

function parseCompanionSnapshot(value: unknown): CompanionSnapshot {
  const record = asRecord(value);
  return {
    ports: parsePorts(record.ports),
    profile: parseProfile(record.profile),
    device: parseDeviceSettings(record.device),
    runtime: parseRuntime(record.runtime),
    lastAsrSeq: asNullableNumber(record.lastAsrSeq),
    lastAsrAdmission:
      asClosed(record.lastAsrAdmission, COMPANION_ASR_ADMISSIONS) ?? "none",
    lastAsrError: asNullableString(record.lastAsrError),
  };
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object"
    ? (value as Record<string, unknown>)
    : {};
}

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function asNullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function asNumber(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asNullableNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function asNullableBoolean(value: unknown): boolean | null {
  return typeof value === "boolean" ? value : null;
}

function asClosed<T extends string>(
  value: unknown,
  allowed: readonly T[],
): T | null {
  return typeof value === "string" && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : null;
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
    listCompanionPorts: async () =>
      parsePorts(await invoke("shurufa_companion_list_ports")),
    captureCompanionTarget: async () => {
      const target = parseTarget(
        await invoke("shurufa_companion_capture_target"),
      );
      if (!target) {
        throw new Error("前台目标不可用");
      }
      return target;
    },
    getCompanionSnapshot: async () =>
      parseCompanionSnapshot(await invoke("shurufa_companion_get_snapshot")),
    saveCompanionProfile: async (draft) => {
      const profile = parseProfile(
        await invoke("shurufa_companion_save_profile", { draft }),
      );
      if (!profile) {
        throw new Error("Companion 配置不可用");
      }
      return profile;
    },
    startCompanionDryRun: async () =>
      parseRuntime(await invoke("shurufa_companion_start_dry_run")),
    enableCompanionLive: async () =>
      parseRuntime(await invoke("shurufa_companion_enable_live")),
    stopCompanion: async () =>
      parseRuntime(await invoke("shurufa_companion_stop")),
    saveCompanionDeviceSettings: async (draft) =>
      parseDeviceSettings(
        await invoke("shurufa_companion_save_device_settings", { draft }),
      ),
    applyCompanionDeviceConfig: async (request) =>
      parseNetwork(
        await invoke("shurufa_companion_apply_device_config", { request }),
      ),
  };
}
