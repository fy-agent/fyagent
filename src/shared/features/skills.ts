import type { SkillAssignments } from "./assignments";
import { PROMPT_APP_IDS, SKILL_TARGET_IDS, SKILL_TARGETS } from "./directory";

/** Interpret the existing native structured-error contract without displaying
 * raw native text, local paths or unvalidated target identifiers. */
export function skillUpdateErrorMessage(error: unknown): string | undefined {
  const raw = error instanceof Error ? error.message : error;
  if (typeof raw !== "string" || raw.length > 4096) return undefined;
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return undefined;
  }
  if (!value || typeof value !== "object" || Array.isArray(value))
    return undefined;
  const record = value as Record<string, unknown>;
  if (record.code === "SKILL_LINK_READ_ONLY") {
    return "检测到 Skills 链接目录，此操作未执行。链接及其目标只读，不会写入或删除；请使用普通目录，或在外部调整链接后刷新。";
  }
  if (record.code === "UPDATE_BACKUP_FAILED") {
    return "旧版本备份失败，更新未执行。请处理备份目录问题后重试。";
  }
  if (
    record.code !== "UPDATE_INCOMPLETE" ||
    !record.context ||
    typeof record.context !== "object" ||
    Array.isArray(record.context)
  )
    return undefined;
  const context = record.context as Record<string, unknown>;
  const targets = (key: string): string[] | undefined => {
    const ids = context[key];
    if (typeof ids !== "string") return undefined;
    const parsed = ids === "" ? [] : ids.split(",").map((id) => id.trim());
    if (
      parsed.length > SKILL_TARGETS.length ||
      new Set(parsed).size !== parsed.length ||
      parsed.some((id) => !SKILL_TARGETS.some((target) => target.id === id))
    )
      return undefined;
    return parsed;
  };
  const applied = targets("applied");
  const failed = targets("failed");
  const conflicted = targets("conflicted");
  if (
    !applied ||
    !failed?.length ||
    !conflicted ||
    conflicted.some((id) => !failed.includes(id) || !applied.includes(id))
  )
    return undefined;
  const labels = (ids: string[]) =>
    ids
      .map((id) => SKILL_TARGETS.find((target) => target.id === id)!.label)
      .join("、");
  const complete = applied.filter((id) => !conflicted.includes(id));
  return [
    "部分目标更新未完成。",
    complete.length ? `已完成：${labels(complete)}。` : "",
    `未完成：${labels(failed)}。`,
    conflicted.length
      ? `${labels(conflicted)} 的后续修改已保留，请确认后再处理。`
      : "旧版本备份已保留，再次更新只处理未完成目标。",
  ].join("");
}

export interface InstalledSkill {
  readOnly?: boolean;
  readOnlyTargets?: string[];
  id: string;
  name: string;
  description?: string;
  directory: string;
  path?: string;
  repoOwner?: string;
  repoName?: string;
  repoBranch?: string;
  readmeUrl?: string;
  apps: SkillAssignments;
  installedAt: number;
  contentHash?: string;
  updatedAt: number;
}

export interface DiscoverableSkill {
  key: string;
  name: string;
  description: string;
  directory: string;
  readmeUrl?: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
}

export const SKILL_DISCOVERY_PAGE_SIZE = 21;
export const SKILL_DISCOVERY_MAX_PAGE_SIZE = 50;
export const SKILLHUB_MARKET_OWNER = "skillhub.cn";
export const SKILLHUB_CATEGORY_ALL = "all" as const;

export const SKILLHUB_OFFICIAL_CATEGORIES = [
  { key: "office-efficiency", name: "办公效率" },
  { key: "content-creation", name: "内容创作" },
  { key: "dev-programming", name: "开发编程" },
  { key: "data-analysis", name: "数据分析" },
  { key: "design-media", name: "设计多媒体" },
  { key: "ai-agent", name: "AI Agent" },
  { key: "knowledge-management", name: "知识管理" },
  { key: "business-ops", name: "商业运营" },
  { key: "education", name: "教育学习" },
  { key: "professional", name: "行业专业" },
  { key: "it-ops-security", name: "IT 运维与安全" },
  { key: "life-service", name: "生活服务" },
] as const;

export type SkillHubCategoryKey =
  (typeof SKILLHUB_OFFICIAL_CATEGORIES)[number]["key"];
export type SkillHubCategoryFilter =
  | typeof SKILLHUB_CATEGORY_ALL
  | SkillHubCategoryKey;

export interface SkillHubCategory {
  key: string;
  name: string;
}

export const SKILLHUB_CATEGORY_TABS: ReadonlyArray<{
  id: SkillHubCategoryFilter;
  label: string;
}> = [
  { id: SKILLHUB_CATEGORY_ALL, label: "全部" },
  ...SKILLHUB_OFFICIAL_CATEGORIES.map((item) => ({
    id: item.key,
    label: item.name,
  })),
];

export type SkillDiscoveryStatus = "all" | "installed" | "uninstalled";

export interface DiscoverableSkillsPage {
  skills: DiscoverableSkill[];
  totalCount: number;
}

export interface DiscoverSkillsPageRequest {
  query: string;
  repo?: string;
  status: SkillDiscoveryStatus;
  limit: number;
  offset: number;
}

export interface SkillHubSkill {
  key: string;
  slug: string;
  name: string;
  description: string;
  directory: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  version?: string;
  ownerName?: string;
  installs?: number;
  downloads?: number;
  homepageUrl: string;
  readmeUrl?: string;
  category?: string;
}

export interface SkillHubSearchResult {
  skills: SkillHubSkill[];
  totalCount: number;
  query: string;
  categories?: SkillHubCategory[];
}

export interface SkillUpdateInfo {
  id: string;
  name: string;
  currentHash?: string;
  remoteHash: string;
}

export interface SkillRepo {
  owner: string;
  name: string;
  branch: string;
  enabled: boolean;
}

export interface UnmanagedSkill {
  readOnly?: boolean;
  directory: string;
  name: string;
  description?: string;
  foundIn: string[];
  path: string;
}

export interface ImportSkillSelection {
  directory: string;
  apps: SkillAssignments;
}

export interface SkillBackupEntry {
  backupId: string;
  backupPath: string;
  createdAt: number;
  skill: InstalledSkill;
}

export interface SkillMigrationResult {
  migratedCount: number;
  skippedCount: number;
  errors: string[];
}

/** Native observations cover all nine SkillTargetId values, including the two
 * existing prompt-directory targets that the seven-target UI does not expose. */
const observedSkillTargetIds = new Set<string>([
  ...SKILL_TARGET_IDS,
  ...PROMPT_APP_IDS.filter((id) => id === "gemini" || id === "hermes"),
]);

export const SKILL_OBSERVATION_PAYLOAD_ERROR =
  "Skills observation response is invalid";

function observedSkillRows(value: unknown): Record<string, unknown>[] {
  if (!Array.isArray(value)) throw new Error(SKILL_OBSERVATION_PAYLOAD_ERROR);
  return value.map((raw: unknown) => {
    if (!raw || typeof raw !== "object" || Array.isArray(raw))
      throw new Error(SKILL_OBSERVATION_PAYLOAD_ERROR);
    const row = raw as Record<string, unknown>;
    // The current native commands always emit this observation. Missing or
    // malformed metadata cannot become an apparently writable legacy row.
    if (typeof row.readOnly !== "boolean")
      throw new Error(SKILL_OBSERVATION_PAYLOAD_ERROR);
    return row;
  });
}

/** Validate fresh observation metadata without changing the base DTO values. */
export function parseObservedInstalledSkills(value: unknown): InstalledSkill[] {
  for (const row of observedSkillRows(value)) {
    if (
      !["id", "name", "directory"].every(
        (key) => typeof row[key] === "string",
      ) ||
      typeof row.installedAt !== "number" ||
      !Number.isSafeInteger(row.installedAt) ||
      typeof row.updatedAt !== "number" ||
      !Number.isSafeInteger(row.updatedAt) ||
      !row.apps ||
      typeof row.apps !== "object" ||
      Array.isArray(row.apps) ||
      !SKILL_TARGET_IDS.every(
        (id) => typeof (row.apps as Record<string, unknown>)[id] === "boolean",
      ) ||
      !Object.values(row.apps).every(
        (enabled) => typeof enabled === "boolean",
      ) ||
      ![
        "description",
        "path",
        "repoOwner",
        "repoName",
        "repoBranch",
        "readmeUrl",
        "contentHash",
      ].every(
        (key) => row[key] === undefined || typeof row[key] === "string",
      ) ||
      !Array.isArray(row.readOnlyTargets) ||
      row.readOnlyTargets.length > observedSkillTargetIds.size ||
      new Set(row.readOnlyTargets).size !== row.readOnlyTargets.length ||
      !row.readOnlyTargets.every(
        (id: unknown) =>
          typeof id === "string" && observedSkillTargetIds.has(id),
      )
    )
      throw new Error(SKILL_OBSERVATION_PAYLOAD_ERROR);
  }
  return value as InstalledSkill[];
}

export function parseObservedUnmanagedSkills(value: unknown): UnmanagedSkill[] {
  for (const row of observedSkillRows(value)) {
    if (
      !["directory", "name", "path"].every(
        (key) => typeof row[key] === "string",
      ) ||
      (row.description !== undefined && typeof row.description !== "string") ||
      !Array.isArray(row.foundIn) ||
      !row.foundIn.every((id: unknown) => typeof id === "string")
    )
      throw new Error(SKILL_OBSERVATION_PAYLOAD_ERROR);
  }
  return value as UnmanagedSkill[];
}
