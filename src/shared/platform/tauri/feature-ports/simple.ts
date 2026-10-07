import { invoke } from "@tauri-apps/api/core";

import type { FeaturePorts } from "../../../features/ports";
import { parseFirstUseGuideState } from "../../../features/first-use-guide";
import {
  parseObservedInstalledSkills,
  parseObservedUnmanagedSkills,
} from "../../../features/skills";
import {
  MCP_IMPORT_SOURCES,
  type McpImportReport,
  type McpImportSourceId,
  type McpServersMap,
} from "../../../features/mcp";

function isMcpImportSource(value: unknown): value is McpImportSourceId {
  return MCP_IMPORT_SOURCES.some((source) => source.id === value);
}

function validateImportSources(sources: McpImportSourceId[]): void {
  if (
    !sources.length ||
    sources.length > MCP_IMPORT_SOURCES.length ||
    new Set(sources).size !== sources.length ||
    !sources.every(isMcpImportSource)
  ) {
    throw new Error("请选择有效且不重复的 MCP 导入来源");
  }
}

function parseMcpImportReport(
  value: unknown,
  sources: McpImportSourceId[],
): McpImportReport {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("MCP 导入结果无效");
  const report = value as Record<string, unknown>;
  if (
    report.contractVersion !== 1 ||
    Object.keys(report).length !== 2 ||
    !Array.isArray(report.sources) ||
    report.sources.length !== sources.length
  )
    throw new Error("MCP 导入结果无效");
  const counts = ["added", "assignmentChanged", "unchanged", "disabledSkipped"];
  for (const [index, raw] of report.sources.entries()) {
    if (!raw || typeof raw !== "object" || Array.isArray(raw))
      throw new Error("MCP 导入结果无效");
    const row = raw as Record<string, unknown>;
    if (
      Object.keys(row).length !== 6 ||
      row.source !== sources[index] ||
      (row.failureCode !== null && row.failureCode !== "source_failed") ||
      !counts.every(
        (key) =>
          typeof row[key] === "number" &&
          Number.isSafeInteger(row[key]) &&
          (row[key] as number) >= 0,
      ) ||
      (row.failureCode !== null && counts.some((key) => row[key] !== 0))
    )
      throw new Error("MCP 导入结果无效");
  }
  return value as McpImportReport;
}

function parseMcpSourceMetadata(value: McpServersMap): McpServersMap {
  for (const server of Object.values(value)) {
    if (
      server.sources !== undefined &&
      (!Array.isArray(server.sources) ||
        !server.sources.every(isMcpImportSource) ||
        new Set(server.sources).size !== server.sources.length)
    )
      throw new Error("MCP 来源记录无效");
  }
  return value;
}

function validateExternalUrl(url: string): void {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error("外部链接无效");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("只允许打开 HTTP(S) 链接");
  }
}

export function createSimpleFeaturePorts(): Pick<
  FeaturePorts,
  "skills" | "mcp" | "settings"
> {
  return {
    skills: {
      getInstalled: async () =>
        parseObservedInstalledSkills(
          await invoke<unknown>("get_installed_skills"),
        ),
      getBackups: () => invoke("get_skill_backups"),
      deleteBackup: (backupId) => invoke("delete_skill_backup", { backupId }),
      install: (skill, currentApp) =>
        invoke("install_skill_unified", { skill, currentApp }),
      uninstall: (id) => invoke("uninstall_skill_unified", { id }),
      restoreBackup: (backupId, currentApp) =>
        invoke("restore_skill_backup", { backupId, currentApp }),
      toggleApp: (id, app, enabled) =>
        invoke("toggle_skill_app", { id, app, enabled }),
      scanUnmanaged: async () =>
        parseObservedUnmanagedSkills(
          await invoke<unknown>("scan_unmanaged_skills"),
        ),
      importFromApps: (imports) =>
        invoke("import_skills_from_apps", { imports }),
      discoverPage: (request) =>
        invoke("discover_available_skills_page", {
          query: request.query,
          repo: request.repo ?? null,
          status: request.status,
          limit: request.limit,
          offset: request.offset,
        }),
      checkUpdates: () => invoke("check_skill_updates"),
      update: (id) => invoke("update_skill", { id }),
      migrateStorage: (target) => invoke("migrate_skill_storage", { target }),
      searchSkillHub: (query, limit, offset, category = "") =>
        invoke("search_skillhub", { query, limit, offset, category }),
      installSkillHub: (slug, currentApp) =>
        invoke("install_skillhub", { slug, currentApp }),
      getRepos: () => invoke("get_skill_repos"),
      addRepo: (repo) => invoke("add_skill_repo", { repo }),
      removeRepo: (owner, name) => invoke("remove_skill_repo", { owner, name }),
      pickZip: () => invoke("open_zip_file_dialog"),
      installFromZip: (filePath, currentApp) =>
        invoke("install_skills_from_zip", { filePath, currentApp }),
    },
    mcp: {
      getAll: async () =>
        parseMcpSourceMetadata(await invoke<McpServersMap>("get_mcp_servers")),
      upsert: (server) => invoke("upsert_mcp_server", { server }),
      delete: (id) => invoke("delete_mcp_server", { id }),
      toggleApp: (serverId, app, enabled) =>
        invoke("toggle_mcp_app", { serverId, app, enabled }),
      importFromApps: async (selection) => {
        const sources = selection
          ? [...selection]
          : MCP_IMPORT_SOURCES.map((source) => source.id);
        validateImportSources(sources);
        const result =
          selection === undefined
            ? await invoke<unknown>("import_mcp_from_apps")
            : await invoke<unknown>("import_mcp_from_apps", { sources });
        return parseMcpImportReport(result, sources);
      },
    },
    settings: {
      getAppVersion: async () => {
        const { readAppVersion } = await import("./application");
        return readAppVersion();
      },
      get: () => invoke("get_settings"),
      save: (settings) => invoke("save_settings", { settings }),
      getFirstUseGuideState: async () =>
        parseFirstUseGuideState(
          await invoke<unknown>("get_first_use_guide_state"),
        ),
      dismissFirstUseGuide: async () => {
        const state = parseFirstUseGuideState(
          await invoke<unknown>("dismiss_first_use_guide"),
        );
        if (state !== "dismissed")
          throw new Error("First-use guide was not dismissed");
        return state;
      },
      openExternal: async (url) => {
        validateExternalUrl(url);
        await invoke("open_external", { url });
      },
    },
  };
}
