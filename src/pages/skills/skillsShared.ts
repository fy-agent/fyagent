import { featureKeys } from "../../shared/features/queries";
import {
  SKILLHUB_MARKET_OWNER,
  SKILLHUB_OFFICIAL_CATEGORIES,
  type DiscoverableSkill,
  type SkillHubSkill,
} from "../../shared/features/types";

export type SkillsTab = "installed" | "discovery";
export type DialogName = "more" | "unmanaged" | "backups" | "settings" | null;

export function formatSkillTimestamp(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "未知";
  return new Date(value * 1000).toLocaleString();
}

export function githubRepoUrl(owner: string, name: string): string | null {
  if (owner.toLowerCase() === SKILLHUB_MARKET_OWNER) return null;
  if (!/^[\w.-]+$/.test(owner) || !/^[\w.-]+$/.test(name)) return null;
  return `https://github.com/${owner}/${name}`;
}

export function isMarketSkill(skill: { repoOwner?: string }): boolean {
  return (skill.repoOwner ?? "").toLowerCase() === SKILLHUB_MARKET_OWNER;
}

export function skillRepoKey(skill: {
  repoOwner: string;
  repoName: string;
}): string {
  return `${skill.repoOwner}/${skill.repoName}`;
}

export function skillDirectoryNote(skill: {
  name: string;
  directory: string;
}): string {
  return skill.directory && skill.directory !== skill.name
    ? skill.directory
    : "";
}

export function skillCardBody(skill: DiscoverableSkill): string {
  const description = skill.description.trim();
  if (description) return description;
  return skillDirectoryNote(skill);
}

export type DiscoverySkill = DiscoverableSkill &
  Partial<
    Pick<
      SkillHubSkill,
      | "slug"
      | "version"
      | "ownerName"
      | "installs"
      | "downloads"
      | "homepageUrl"
      | "category"
    >
  >;

export function skillHubCategoryLabel(key: string | undefined): string {
  if (!key) return "";
  return (
    SKILLHUB_OFFICIAL_CATEGORIES.find((item) => item.key === key)?.name ?? ""
  );
}

export function skillDetailMeta(skill: DiscoverySkill): string {
  if (isMarketSkill(skill)) {
    return [
      "Skill 市场",
      skillHubCategoryLabel(skill.category),
      skill.slug ?? skill.repoName,
      skill.ownerName,
      skill.version ? `v${skill.version}` : null,
      typeof skill.installs === "number"
        ? `${skill.installs.toLocaleString()} 次安装`
        : null,
    ]
      .filter(Boolean)
      .join(" · ");
  }
  return skillRepoKey(skill);
}

export function skillDocsAction(
  skill: DiscoverySkill,
): { url: string; label: "说明" | "仓库" | "主页" } | null {
  if (isMarketSkill(skill)) {
    const url = skill.homepageUrl ?? skill.readmeUrl;
    return url ? { url, label: "主页" } : null;
  }
  const repoUrl = githubRepoUrl(skill.repoOwner, skill.repoName);
  if (skill.readmeUrl) {
    return {
      url: skill.readmeUrl,
      label: /SKILL\.md|\/blob\//i.test(skill.readmeUrl) ? "说明" : "仓库",
    };
  }
  return repoUrl ? { url: repoUrl, label: "仓库" } : null;
}

export const INSTALLED_SPLIT_LABELS = [
  "调整列表与详情的宽度",
  "调整详情与分配的宽度",
];

export const invalidations = [
  featureKeys.skills,
  featureKeys.skillBackups,
  featureKeys.skillDiscovery,
  featureKeys.skillUnmanaged,
];
