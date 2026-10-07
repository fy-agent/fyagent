import type { SkillAssignments } from "./assignments";
import { SKILL_TARGETS } from "./directory";

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
