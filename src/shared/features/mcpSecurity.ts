const REDACTED_SECRET = "••••••";

const SENSITIVE_QUERY_KEYS = new Set([
  "key",
  "ak",
  "api_key",
  "apikey",
  "api-key",
  "token",
  "access_token",
  "access-token",
  "secret",
  "secretkey",
  "secret_key",
  "secret-key",
  "password",
  "authorization",
  "auth",
  "pat",
  "client_secret",
  "client-secret",
  "app_secret",
  "app-secret",
]);

const SENSITIVE_ARG_FLAGS = new Set([
  "-s",
  "--secret",
  "--app-secret",
  "--client-secret",
  "--token",
  "--api-key",
  "--apikey",
  "--key",
  "--password",
  "--pat",
  "--access-token",
  "--authorization",
]);

const QUERY_FALLBACK_PATTERN =
  /([?&](?:key|ak|api[_-]?key|token|access[_-]?token|secret|password|authorization|auth|pat|client[_-]?secret|app[_-]?secret)=)([^&]*)/gi;

function normalizeFlag(value: string): string {
  return value.trim().toLocaleLowerCase();
}

export function isSensitiveQueryKey(key: string): boolean {
  return SENSITIVE_QUERY_KEYS.has(normalizeFlag(key));
}

export function isSensitiveArgFlag(flag: string): boolean {
  return SENSITIVE_ARG_FLAGS.has(normalizeFlag(flag));
}

export function mcpUrlSearchToken(url: string): string {
  try {
    const parsed = new URL(url);
    return `${parsed.origin}${parsed.pathname}`;
  } catch {
    return url.split(/[?#]/, 1)[0] ?? url;
  }
}

export function redactMcpUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const pairs = [...parsed.searchParams.entries()];
    if (pairs.length === 0) {
      return `${parsed.origin}${parsed.pathname}${parsed.hash}`;
    }
    const query = pairs
      .map(([key, value]) => {
        const encodedValue = isSensitiveQueryKey(key)
          ? REDACTED_SECRET
          : encodeURIComponent(value);
        return `${encodeURIComponent(key)}=${encodedValue}`;
      })
      .join("&");
    return `${parsed.origin}${parsed.pathname}?${query}${parsed.hash}`;
  } catch {
    return url.replace(QUERY_FALLBACK_PATTERN, `$1${REDACTED_SECRET}`);
  }
}

export function redactMcpArgs(args: readonly string[]): string[] {
  const redacted: string[] = [];
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    const separator = argument.indexOf("=");
    if (separator > 0 && isSensitiveArgFlag(argument.slice(0, separator))) {
      redacted.push(`${argument.slice(0, separator)}=${REDACTED_SECRET}`);
      continue;
    }
    redacted.push(argument);
    if (isSensitiveArgFlag(argument) && index + 1 < args.length) {
      redacted.push(REDACTED_SECRET);
      index += 1;
    }
  }
  return redacted;
}

function normalizeStdioCommand(
  command: string | undefined,
  args: readonly string[],
): string {
  if (command === "cmd" && args.includes("npx")) return "npx";
  return command ?? "";
}

function extractLaunchPackage(
  command: string | undefined,
  args: readonly string[],
): string {
  if (command === "uvx") return args[0] ?? "";
  const skipIndex = args.indexOf("-y");
  if (skipIndex >= 0) return args[skipIndex + 1] ?? "";
  return args.find((argument) => argument.startsWith("@")) ?? args[0] ?? "";
}

export function mcpRecipeIdentity(spec: {
  type?: string;
  command?: string;
  args?: string[];
  url?: string;
}): string {
  const type =
    spec.type === "http" || spec.type === "sse" ? spec.type : "stdio";
  if (type !== "stdio") {
    return JSON.stringify({
      type,
      url: spec.url ? mcpUrlSearchToken(spec.url) : "",
    });
  }
  const args = spec.args ?? [];
  return JSON.stringify({
    type: "stdio",
    command: normalizeStdioCommand(spec.command, args),
    packageName: extractLaunchPackage(spec.command, args),
  });
}
