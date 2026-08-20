import { vi } from "vitest";
import { server } from "./server";

const TAURI_ENDPOINT = "http://tauri.local";
const invokedCommands: string[] = [];
const emittedEvents: Array<{ event: string; payload: unknown }> = [];
let tauriRequestHeaders: HeadersInit | undefined;

export const clearTauriInvocations = () => {
  invokedCommands.length = 0;
  emittedEvents.length = 0;
  tauriRequestHeaders = undefined;
};

export const getTauriInvocations = (): readonly string[] => invokedCommands;
export const getEmittedTauriEvents = (): ReadonlyArray<{
  event: string;
  payload: unknown;
}> => emittedEvents;

export const setTauriRequestHeaders = (headers: HeadersInit | undefined) => {
  tauriRequestHeaders = headers;
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (command: string, payload: Record<string, unknown> = {}) => {
    invokedCommands.push(command);
    const response = await fetch(`${TAURI_ENDPOINT}/${command}`, {
      method: "POST",
      headers: tauriRequestHeaders ?? { "Content-Type": "application/json" },
      body: JSON.stringify(payload ?? {}),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(text || `Invoke failed for ${command}`);
    }

    const text = await response.text();
    if (!text) return undefined;
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  },
}));

const listeners = new Map<string, Set<(event: { payload: unknown }) => void>>();

const ensureListenerSet = (event: string) => {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  return listeners.get(event)!;
};

export const emitTauriEvent = (event: string, payload: unknown) => {
  const handlers = listeners.get(event);
  handlers?.forEach((handler) => handler({ payload }));
};

vi.mock("@tauri-apps/api/event", () => ({
  emit: async (event: string, payload?: unknown) => {
    emittedEvents.push({ event, payload });
  },
  listen: async (
    event: string,
    handler: (event: { payload: unknown }) => void,
  ) => {
    const set = ensureListenerSet(event);
    set.add(handler);
    return () => {
      set.delete(handler);
    };
  },
}));

// Ensure the MSW server is referenced so tree shaking doesn't remove imports
void server;

vi.mock("@tauri-apps/api/path", () => ({
  homeDir: async () => "/Users/mock",
  join: async (...segments: string[]) => segments.join("/"),
}));

const mockCurrentWindow = {
  onFocusChanged:
    async (_handler: (event: { payload: boolean }) => void) => () =>
      undefined,
};

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => mockCurrentWindow,
}));
