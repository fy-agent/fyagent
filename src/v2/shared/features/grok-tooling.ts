export const GROK_DISTRIBUTION_OWNERS = [
  "native_internal",
  "official_npm",
] as const;
export type GrokDistributionOwner = (typeof GROK_DISTRIBUTION_OWNERS)[number];

export interface GrokToolSnapshot {
  localVersion: string | null;
  latestVersion: string | null;
  distributionOwner: GrokDistributionOwner | null;
  latestSource: GrokDistributionOwner | null;
  installedButBroken: boolean;
  error: string | null;
}

export interface GrokToolingPort {
  getSnapshot(): Promise<GrokToolSnapshot>;
  installOfficialNpm(): Promise<void>;
}

const GROK_TOOL_NAME = "grok";
const FORBIDDEN_GROK_WIRE = [
  "http://",
  "https://",
  "token",
  "secret",
  "apiKey",
  "api_key",
  "packageFormat",
] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isOneOf<T extends string>(
  value: unknown,
  candidates: readonly T[],
): value is T {
  return typeof value === "string" && candidates.includes(value as T);
}

function optionalOwner(value: unknown): GrokDistributionOwner | null {
  if (value === undefined || value === null) return null;
  if (!isOneOf(value, GROK_DISTRIBUTION_OWNERS)) {
    throw new Error("Grok 安装状态不可用");
  }
  return value;
}

function optionalString(value: unknown): string | null {
  if (value === null) return null;
  if (typeof value !== "string") {
    throw new Error("Grok 安装状态不可用");
  }
  return value;
}

export function parseGrokToolSnapshot(value: unknown): GrokToolSnapshot {
  if (!Array.isArray(value)) {
    throw new Error("Grok 安装状态不可用");
  }
  const grok = value.find(
    (item) => isRecord(item) && item.name === GROK_TOOL_NAME,
  );
  if (!grok || !isRecord(grok)) {
    return {
      localVersion: null,
      latestVersion: null,
      distributionOwner: null,
      latestSource: null,
      installedButBroken: false,
      error: null,
    };
  }
  const encoded = JSON.stringify(grok).toLowerCase();
  if (FORBIDDEN_GROK_WIRE.some((needle) => encoded.includes(needle))) {
    throw new Error("Grok 安装状态不可用");
  }
  const required = [
    "name",
    "version",
    "latest_version",
    "error",
    "installed_but_broken",
  ];
  const allowed = new Set([...required, "distribution_owner", "latest_source"]);
  if (
    !required.every((key) => Object.prototype.hasOwnProperty.call(grok, key)) ||
    Object.keys(grok).some((key) => !allowed.has(key)) ||
    grok.name !== GROK_TOOL_NAME ||
    typeof grok.installed_but_broken !== "boolean"
  ) {
    throw new Error("Grok 安装状态不可用");
  }
  return {
    localVersion: optionalString(grok.version),
    latestVersion: optionalString(grok.latest_version),
    distributionOwner: optionalOwner(grok.distribution_owner),
    latestSource: optionalOwner(grok.latest_source),
    installedButBroken: grok.installed_but_broken,
    error: optionalString(grok.error),
  };
}

export function grokOwnerCopy(owner: GrokDistributionOwner | null): string {
  switch (owner) {
    case "native_internal":
      return "官方命令行";
    case "official_npm":
      return "官方 npm 包";
    default:
      return "未确认";
  }
}

export function grokLatestLabel(source: GrokDistributionOwner | null): string {
  switch (source) {
    case "native_internal":
      return "官方命令行最新";
    case "official_npm":
      return "官方 npm 最新";
    default:
      return "最新版本";
  }
}
