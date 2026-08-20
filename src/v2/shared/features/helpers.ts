import { currentMcpLaunchPlatform, type McpLaunchPlatform } from "./mcpLaunch";
import { mcpUrlSearchToken, redactMcpArgs } from "./mcpSecurity";
import {
  SKILLHUB_MARKET_OWNER,
  SKILL_TARGET_IDS,
  type DiscoverableSkill,
  type InstalledSkill,
  type McpServer,
  type McpServerSpec,
  type SkillTargetId,
} from "./types";

function rawErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** A message composed locally from validated user input and safe to display. */
export class UserFacingError extends Error {
  override readonly name = "UserFacingError";
}

export function errorMessage(error: unknown): string {
  const message = rawErrorMessage(error);
  if (error instanceof UserFacingError) return message;
  if (message.includes("仅在 FyAgent 桌面应用中可用")) {
    return "此功能仅在 FyAgent 桌面应用中可用。";
  }
  return "请稍后重试。";
}

export function isNativeOnlyError(error: unknown): boolean {
  return rawErrorMessage(error).includes("仅在 FyAgent 桌面应用中可用");
}

export function sanitizeMcpConfigurationError(error: unknown): string {
  const message = rawErrorMessage(error);
  const importConflict = message.match(
    /配置冲突；未合并 (claude|codex|gemini|grokbuild|opencode|hermes|workbuddy|qoderwork|trae-work) 分配/i,
  );
  if (importConflict) {
    const appLabels: Record<string, string> = {
      claude: "Claude Code",
      codex: "Codex",
      gemini: "Gemini",
      grokbuild: "Grok Build",
      opencode: "OpenCode",
      hermes: "Hermes",
      workbuddy: "WorkBuddy",
      qoderwork: "QoderWork CN",
      "trae-work": "TRAE Work CN",
    };
    const appLabel = appLabels[importConflict[1].toLocaleLowerCase()];
    return `检测到同名 MCP 服务器的配置冲突，未合并 ${appLabel} 分配；请统一两端配置或更改服务器 ID`;
  }
  if (message.includes("配置冲突")) {
    return "检测到同名 MCP 服务器的配置冲突；请统一两端配置或更改服务器 ID";
  }
  if (
    /env|header|authorization|token|secret|password|api[-_ ]?key/i.test(message)
  ) {
    return "MCP 配置中的敏感字段未通过校验，请检查对应字段格式";
  }
  if (/\burl\b/i.test(message)) {
    return "MCP 配置中的 URL 未通过校验，请检查连接地址";
  }
  if (/\b(command|args?|cwd|type|transport)\b/i.test(message)) {
    return "MCP 配置中的启动字段未通过校验，请检查传输类型与命令";
  }
  return "MCP 配置保存失败，请检查服务器字段";
}

export function convergeSelection<T extends { id: string }>(
  items: readonly T[],
  selectedId: string | null,
): string | null {
  if (selectedId && items.some((item) => item.id === selectedId)) {
    return selectedId;
  }
  return items[0]?.id ?? null;
}

export function skillInstallPath(
  skill: Pick<InstalledSkill, "directory" | "path">,
): string {
  const path = skill.path?.trim();
  return path ? path : skill.directory;
}

const SKILL_DESTINATION_ROOT = {
  qoderwork: "~/.qoderworkcn/skills",
  "trae-work": "~/.trae-cn/skills",
  workbuddy: "~/.workbuddy/skills",
  grokbuild: "~/.grok/skills",
  codex: "~/.codex/skills",
  claude: "~/.claude/skills",
  opencode: "~/.config/opencode/skills",
} as const satisfies Record<SkillTargetId, string>;

export function skillInstallDestination(
  target: SkillTargetId,
  directory?: string,
): string {
  const root = SKILL_DESTINATION_ROOT[target];
  const name = directory?.trim().replace(/^[/\\]+|[/\\]+$/g, "");
  return name ? `${root}/${name}` : root;
}

export function mcpInstallDestination(
  target: SkillTargetId,
  platform: McpLaunchPlatform = currentMcpLaunchPlatform(),
): string {
  switch (target) {
    case "qoderwork":
      return "~/.qoderworkcn/mcp.json";
    case "trae-work":
      return platform === "windows"
        ? "%APPDATA%\\TRAE SOLO CN\\User\\mcp.json"
        : "~/Library/Application Support/TRAE SOLO CN/User/mcp.json";
    case "workbuddy":
      return "~/.workbuddy/mcp.json";
    case "grokbuild":
      return "~/.grok/config.toml";
    case "codex":
      return "~/.codex/config.toml";
    case "claude":
      return "~/.claude.json";
    case "opencode":
      return "~/.config/opencode/opencode.json";
  }
}

export function mcpInstallDirectory(
  spec: Pick<McpServerSpec, "command" | "cwd">,
): string | null {
  const cwd = spec.cwd?.trim();
  if (cwd) return cwd;
  const command = spec.command?.trim();
  if (!command || !isAbsoluteFilesystemPath(command)) return null;
  return parentDirectory(command);
}

function isAbsoluteFilesystemPath(value: string): boolean {
  return (
    /^[a-zA-Z]:[\\/]/.test(value) ||
    value.startsWith("\\\\") ||
    value.startsWith("/")
  );
}

function parentDirectory(value: string): string {
  const trimmed = value.replace(/[\\/]+$/, "");
  const index = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  if (index < 0) return trimmed;
  if (trimmed.startsWith("/") && index === 0) return "/";
  if (/^[a-zA-Z]:[\\/]/.test(trimmed) && index === 2) {
    return trimmed.slice(0, 3);
  }
  return trimmed.slice(0, index);
}

export function buildSkillSearchText(skill: InstalledSkill): string {
  return [
    skill.name,
    skill.id,
    skill.description,
    skill.directory,
    skill.repoOwner,
    skill.repoName,
    skill.repoOwner && skill.repoName
      ? `${skill.repoOwner}/${skill.repoName}`
      : undefined,
  ]
    .filter(Boolean)
    .join("\n")
    .toLocaleLowerCase();
}

export function buildMcpSearchText(server: McpServer): string {
  const spec = server.server;
  return [
    server.id,
    server.name,
    server.description,
    ...(server.tags ?? []),
    spec.type,
    spec.command,
    ...redactMcpArgs(spec.args ?? []),
    spec.cwd,
    spec.url ? mcpUrlSearchToken(spec.url) : undefined,
    server.homepage,
    server.docs,
    server.source,
  ]
    .filter((value): value is string => typeof value === "string")
    .join("\n")
    .toLocaleLowerCase();
}

function directoryTail(directory: string): string {
  return directory.split(/[/\\]/).filter(Boolean).at(-1)?.toLowerCase() ?? "";
}

export function isDiscoverableInstalled(
  discoverable: DiscoverableSkill,
  installed: readonly InstalledSkill[],
): boolean {
  const owner = discoverable.repoOwner.toLowerCase();
  const name = discoverable.repoName.toLowerCase();
  if (owner === SKILLHUB_MARKET_OWNER) {
    return installed.some(
      (skill) =>
        skill.id.toLowerCase() === `skillhub:${name}` ||
        ((skill.repoOwner ?? "").toLowerCase() === SKILLHUB_MARKET_OWNER &&
          (skill.repoName ?? "").toLowerCase() === name),
    );
  }
  const tail = directoryTail(discoverable.directory);
  return installed.some(
    (skill) =>
      directoryTail(skill.directory) === tail &&
      (skill.repoOwner ?? "").toLowerCase() === owner &&
      (skill.repoName ?? "").toLowerCase() === name,
  );
}

export interface LineMapResult {
  value: Record<string, string>;
  errors: string[];
}

export function parseKeyValueLines(
  text: string,
  kind: "env" | "headers",
): LineMapResult {
  const value: Record<string, string> = {};
  const errors: string[] = [];
  text.split(/\r?\n/).forEach((rawLine, index) => {
    if (!rawLine.trim()) return;
    const equals = rawLine.indexOf("=");
    const colon = kind === "headers" ? rawLine.indexOf(":") : -1;
    const candidates = [equals, colon].filter((position) => position > 0);
    const separator = candidates.length ? Math.min(...candidates) : -1;
    if (separator < 1 || !rawLine.slice(0, separator).trim()) {
      errors.push(`第 ${index + 1} 行格式无效`);
      return;
    }
    value[rawLine.slice(0, separator).trim()] = rawLine
      .slice(separator + 1)
      .trim();
  });
  return { value, errors };
}

export function parseAdvancedServerJson(text: string): McpServerSpec {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new UserFacingError("JSON 格式无效。请检查后重试。");
  }
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new UserFacingError("请填写单个 MCP 服务配置。");
  }
  if ("mcpServers" in parsed) {
    throw new UserFacingError("请填写单个 MCP 服务配置，而不是完整配置列表。");
  }
  return parsed as McpServerSpec;
}

export function overlayKnownMcpFields(
  original: McpServerSpec,
  known: McpServerSpec,
): McpServerSpec {
  const result = { ...original };
  for (const key of [
    "type",
    "command",
    "args",
    "env",
    "cwd",
    "url",
    "headers",
  ] as const) {
    delete result[key];
    const value = known[key];
    if (value !== undefined) {
      Object.assign(result, { [key]: value });
    }
  }
  return result;
}

export async function runSequentialBulk<T>(
  ids: readonly string[],
  operation: (id: string) => Promise<T>,
  onProgress?: (completed: number, total: number) => void,
): Promise<{
  successes: string[];
  failures: Array<{ id: string; error: string }>;
}> {
  const successes: string[] = [];
  const failures: Array<{ id: string; error: string }> = [];
  for (const [index, id] of ids.entries()) {
    try {
      await operation(id);
      successes.push(id);
    } catch (error) {
      failures.push({ id, error: errorMessage(error) });
    }
    onProgress?.(index + 1, ids.length);
  }
  return { successes, failures };
}

export function supportedFoundIn(foundIn: readonly string[]): SkillTargetId[] {
  const normalized = new Set(foundIn.map((value) => value.toLowerCase()));
  return SKILL_TARGET_IDS.filter((id) => normalized.has(id));
}
