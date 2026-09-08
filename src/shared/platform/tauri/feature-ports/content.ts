import { invoke } from "@tauri-apps/api/core";

import type { FeaturePorts } from "../../../features/ports";
import {
  HERMES_MEMORY_KINDS,
  MEMORY_DOCUMENT_IDS,
  PROMPT_APP_IDS,
  type DailyMemoryFileInfo,
  type DailyMemorySearchResult,
  type HermesMemoryKind,
  type HermesMemoryLimits,
  type ManagedPrompt,
  type MemoryDocumentId,
  type OpenClawDirectory,
  type PromptAppId,
} from "../../../features/types";
import { hasExactKeys, isOneOf, isRecord } from "./validation";

const MEMORY_DOCUMENT_TARGETS = {
  "openclaw-memory": { source: "openclaw", filename: "MEMORY.md" },
  "openclaw-user": { source: "openclaw", filename: "USER.md" },
  "hermes-memory": { source: "hermes", kind: "memory" },
  "hermes-user": { source: "hermes", kind: "user" },
} as const satisfies Record<
  MemoryDocumentId,
  | { source: "openclaw"; filename: "MEMORY.md" | "USER.md" }
  | { source: "hermes"; kind: HermesMemoryKind }
>;

function hasOnlyKeys(
  value: Record<string, unknown>,
  keys: readonly string[],
): boolean {
  const allowed = new Set(keys);
  return Object.keys(value).every((key) => allowed.has(key));
}

function assertPromptAppId(app: PromptAppId): PromptAppId {
  if (!isOneOf(app, PROMPT_APP_IDS))
    throw new Error("Prompt application is invalid");
  return app;
}

function assertPromptId(value: unknown): string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.trim() !== value ||
    [...value].some((character) => {
      const code = character.charCodeAt(0);
      return code <= 31 || code === 127;
    })
  )
    throw new Error("Prompt identifier is invalid");
  return value;
}

function isOptionalTimestamp(value: unknown): value is number | undefined {
  return (
    value === undefined ||
    (typeof value === "number" && Number.isSafeInteger(value) && value >= 0)
  );
}

function parseManagedPrompt(
  value: unknown,
  expectedId?: string,
): ManagedPrompt {
  if (
    !isRecord(value) ||
    !hasOnlyKeys(value, [
      "id",
      "name",
      "content",
      "description",
      "enabled",
      "createdAt",
      "updatedAt",
    ]) ||
    typeof value.name !== "string" ||
    typeof value.content !== "string" ||
    (value.description !== undefined &&
      typeof value.description !== "string") ||
    typeof value.enabled !== "boolean" ||
    !isOptionalTimestamp(value.createdAt) ||
    !isOptionalTimestamp(value.updatedAt)
  )
    throw new Error("Prompt data is unavailable");

  const id = assertPromptId(value.id);
  if (expectedId !== undefined && id !== expectedId)
    throw new Error("Prompt data is unavailable");

  return {
    id,
    name: value.name,
    content: value.content,
    ...(value.description === undefined
      ? {}
      : { description: value.description }),
    enabled: value.enabled,
    ...(value.createdAt === undefined ? {} : { createdAt: value.createdAt }),
    ...(value.updatedAt === undefined ? {} : { updatedAt: value.updatedAt }),
  };
}

function assertManagedPrompt(prompt: ManagedPrompt): ManagedPrompt {
  const parsed = parseManagedPrompt(prompt);
  if (parsed.name.trim().length === 0)
    throw new Error("Prompt name is required");
  return parsed;
}

function parsePromptCollection(value: unknown): ManagedPrompt[] {
  if (!isRecord(value)) throw new Error("Prompt data is unavailable");
  return Object.entries(value).map(([id, prompt]) =>
    parseManagedPrompt(prompt, assertPromptId(id)),
  );
}

function parseNullableContent(value: unknown, message: string): string | null {
  if (value !== null && typeof value !== "string") throw new Error(message);
  return value;
}

function parseImportedPromptId(value: unknown): string {
  try {
    return assertPromptId(value);
  } catch {
    throw new Error("Imported Prompt data is unavailable");
  }
}

function assertMemoryDocumentId(id: MemoryDocumentId): MemoryDocumentId {
  if (!isOneOf(id, MEMORY_DOCUMENT_IDS))
    throw new Error("Memory document is invalid");
  return id;
}

function assertHermesMemoryKind(kind: HermesMemoryKind): HermesMemoryKind {
  if (!isOneOf(kind, HERMES_MEMORY_KINDS))
    throw new Error("Hermes memory kind is invalid");
  return kind;
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function parseHermesMemoryLimits(value: unknown): HermesMemoryLimits {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["memory", "user", "memoryEnabled", "userEnabled"]) ||
    !isNonNegativeInteger(value.memory) ||
    !isNonNegativeInteger(value.user) ||
    typeof value.memoryEnabled !== "boolean" ||
    typeof value.userEnabled !== "boolean"
  )
    throw new Error("Hermes memory limits are unavailable");

  return {
    memory: value.memory,
    user: value.user,
    memoryEnabled: value.memoryEnabled,
    userEnabled: value.userEnabled,
  };
}

function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

function assertDailyMemoryFilename(value: unknown): string {
  if (typeof value !== "string")
    throw new Error("Daily memory filename is invalid");
  const match = /^(\d{4})-(\d{2})-(\d{2})\.md$/.exec(value);
  if (!match) throw new Error("Daily memory filename is invalid");

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const daysInMonth = [
    31,
    isLeapYear(year) ? 29 : 28,
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ];
  if (
    year < 1 ||
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > daysInMonth[month - 1]
  )
    throw new Error("Daily memory filename is invalid");
  return value;
}

function parseDailyMemoryFileInfo(value: unknown): DailyMemoryFileInfo {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "filename",
      "date",
      "sizeBytes",
      "modifiedAt",
      "preview",
    ]) ||
    typeof value.date !== "string" ||
    !isNonNegativeInteger(value.sizeBytes) ||
    !isNonNegativeInteger(value.modifiedAt) ||
    typeof value.preview !== "string"
  )
    throw new Error("Daily memory list is unavailable");

  const filename = assertDailyMemoryFilename(value.filename);
  if (value.date !== filename.slice(0, -3))
    throw new Error("Daily memory list is unavailable");
  return {
    filename,
    date: value.date,
    sizeBytes: value.sizeBytes,
    modifiedAt: value.modifiedAt,
    preview: value.preview,
  };
}

function parseDailyMemoryFiles(value: unknown): DailyMemoryFileInfo[] {
  if (!Array.isArray(value))
    throw new Error("Daily memory list is unavailable");
  return value.map(parseDailyMemoryFileInfo);
}

function parseDailyMemorySearchResult(value: unknown): DailyMemorySearchResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "filename",
      "date",
      "sizeBytes",
      "modifiedAt",
      "snippet",
      "matchCount",
    ]) ||
    typeof value.date !== "string" ||
    !isNonNegativeInteger(value.sizeBytes) ||
    !isNonNegativeInteger(value.modifiedAt) ||
    typeof value.snippet !== "string" ||
    !isNonNegativeInteger(value.matchCount)
  )
    throw new Error("Daily memory search is unavailable");

  const filename = assertDailyMemoryFilename(value.filename);
  if (value.date !== filename.slice(0, -3))
    throw new Error("Daily memory search is unavailable");
  return {
    filename,
    date: value.date,
    sizeBytes: value.sizeBytes,
    modifiedAt: value.modifiedAt,
    snippet: value.snippet,
    matchCount: value.matchCount,
  };
}

function parseDailyMemorySearchResults(
  value: unknown,
): DailyMemorySearchResult[] {
  if (!Array.isArray(value))
    throw new Error("Daily memory search is unavailable");
  return value.map(parseDailyMemorySearchResult);
}

function assertSearchQuery(query: string): string {
  if (typeof query !== "string")
    throw new Error("Daily memory search query is invalid");
  return query;
}

function assertOpenClawDirectory(subdir: OpenClawDirectory): OpenClawDirectory {
  if (subdir !== "workspace" && subdir !== "memory")
    throw new Error("OpenClaw directory is invalid");
  return subdir;
}

export function createContentFeaturePorts(): Pick<
  FeaturePorts,
  "prompts" | "memory"
> {
  return {
    prompts: {
      getAll: async (app) => {
        const safeApp = assertPromptAppId(app);
        return parsePromptCollection(
          await invoke<unknown>("get_prompts", { app: safeApp }),
        );
      },
      getCurrentFileContent: async (app) => {
        const safeApp = assertPromptAppId(app);
        return parseNullableContent(
          await invoke<unknown>("get_current_prompt_file_content", {
            app: safeApp,
          }),
          "Prompt live file is unavailable",
        );
      },
      upsert: async (app, prompt) => {
        const safeApp = assertPromptAppId(app);
        const safePrompt = assertManagedPrompt(prompt);
        await invoke("upsert_prompt", {
          app: safeApp,
          id: safePrompt.id,
          prompt: safePrompt,
        });
      },
      delete: async (app, id) => {
        await invoke("delete_prompt", {
          app: assertPromptAppId(app),
          id: assertPromptId(id),
        });
      },
      enable: async (app, id) => {
        await invoke("enable_prompt", {
          app: assertPromptAppId(app),
          id: assertPromptId(id),
        });
      },
      importFromFile: async (app) =>
        parseImportedPromptId(
          await invoke<unknown>("import_prompt_from_file", {
            app: assertPromptAppId(app),
          }),
        ),
    },
    memory: {
      readDocument: async (id) => {
        const target = MEMORY_DOCUMENT_TARGETS[assertMemoryDocumentId(id)];
        if (target.source === "openclaw") {
          return parseNullableContent(
            await invoke<unknown>("read_workspace_file", {
              filename: target.filename,
            }),
            "OpenClaw memory document is unavailable",
          );
        }
        const content = await invoke<unknown>("get_hermes_memory", {
          kind: target.kind,
        });
        if (typeof content !== "string")
          throw new Error("Hermes memory document is unavailable");
        return content;
      },
      writeDocument: async (id, content) => {
        if (typeof content !== "string")
          throw new Error("Memory document content is invalid");
        const target = MEMORY_DOCUMENT_TARGETS[assertMemoryDocumentId(id)];
        if (target.source === "openclaw") {
          await invoke("write_workspace_file", {
            filename: target.filename,
            content,
          });
          return;
        }
        await invoke("set_hermes_memory", { kind: target.kind, content });
      },
      getHermesLimits: async () =>
        parseHermesMemoryLimits(
          await invoke<unknown>("get_hermes_memory_limits"),
        ),
      setHermesEnabled: async (kind, enabled) => {
        if (typeof enabled !== "boolean")
          throw new Error("Hermes memory enabled state is invalid");
        await invoke("set_hermes_memory_enabled", {
          kind: assertHermesMemoryKind(kind),
          enabled,
        });
      },
      listDailyFiles: async () =>
        parseDailyMemoryFiles(await invoke<unknown>("list_daily_memory_files")),
      readDailyFile: async (filename) =>
        parseNullableContent(
          await invoke<unknown>("read_daily_memory_file", {
            filename: assertDailyMemoryFilename(filename),
          }),
          "Daily memory file is unavailable",
        ),
      writeDailyFile: async (filename, content) => {
        if (typeof content !== "string")
          throw new Error("Daily memory content is invalid");
        await invoke("write_daily_memory_file", {
          filename: assertDailyMemoryFilename(filename),
          content,
        });
      },
      deleteDailyFile: async (filename) => {
        await invoke("delete_daily_memory_file", {
          filename: assertDailyMemoryFilename(filename),
        });
      },
      searchDailyFiles: async (query) =>
        parseDailyMemorySearchResults(
          await invoke<unknown>("search_daily_memory_files", {
            query: assertSearchQuery(query),
          }),
        ),
      openOpenClawDirectory: async (subdir) => {
        const opened = await invoke<unknown>("open_workspace_directory", {
          subdir: assertOpenClawDirectory(subdir),
        });
        if (opened !== true)
          throw new Error("OpenClaw directory could not be opened");
      },
    },
  };
}
