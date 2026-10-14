export const REDACTED_SECRET = "••••••";

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
  "proxy-authorization",
  "cookie",
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
  "--passwd",
  "--passphrase",
  "--pat",
  "--access-token",
  "--authorization",
]);

const QUERY_FALLBACK_PATTERN =
  /([?&](?:key|ak|api[_-]?key|token|access[_-]?token|secret|password|authorization|auth|pat|client[_-]?secret|app[_-]?secret)=)([^&#]*)/gi;

function normalizeFlag(value: string): string {
  return value.trim().replace(/[_-]/g, "").toLowerCase();
}

function decodeParameterName(key: string): string {
  try {
    return decodeURIComponent(key.replace(/\+/g, " "));
  } catch {
    return key;
  }
}

function isSensitiveHeaderName(name: string): boolean {
  const normalized = normalizeFlag(name);
  return (
    isSensitiveQueryKey(name) ||
    /^(?:proxy)?authorization$|^(?:set)?cookie$|^auth$|^xauth/.test(normalized)
  );
}

export function isSensitiveQueryKey(key: string): boolean {
  return (
    [...SENSITIVE_QUERY_KEYS].some(
      (candidate) => normalizeFlag(candidate) === normalizeFlag(key),
    ) ||
    /[_-](?:pass|pwd|creds|pat)$/i.test(key.trim()) ||
    /(?:token|key|passphrase)$/.test(normalizeFlag(key)) ||
    /(?:secret|password|passwd|credential|privatekey|bearertoken)/.test(
      normalizeFlag(key),
    )
  );
}

export function isSensitiveArgFlag(flag: string): boolean {
  return (
    SENSITIVE_ARG_FLAGS.has(flag.trim().toLowerCase()) ||
    (flag.startsWith("--") && isSensitiveHeaderName(flag.slice(2)))
  );
}

export function mcpUrlSearchToken(url: string): string {
  try {
    const parsed = new URL(url);
    return `${parsed.origin}${parsed.pathname}`;
  } catch {
    return url.split(/[?#]/, 1)[0] ?? url;
  }
}

function redactMcpUrlParameters(value: string): string {
  return value.replace(
    /([?&#])([^=&#]+)=([^&#]*)/g,
    (pair, separator: string, key: string) =>
      isSensitiveHeaderName(decodeParameterName(key))
        ? `${separator}${key}=${REDACTED_SECRET}`
        : pair,
  );
}

export function redactMcpUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const pairs = [...parsed.searchParams.entries()];
    const fragment = redactMcpUrlParameters(parsed.hash);
    if (pairs.length === 0) {
      return `${parsed.origin}${parsed.pathname}${fragment}`;
    }
    const query = pairs
      .map(([key, value]) => {
        const encodedValue = isSensitiveHeaderName(key)
          ? REDACTED_SECRET
          : encodeURIComponent(value);
        return `${encodeURIComponent(key)}=${encodedValue}`;
      })
      .join("&");
    return `${parsed.origin}${parsed.pathname}?${query}${fragment}`;
  } catch {
    return redactMcpUrlParameters(
      url
        .replace(/(\/\/)[^/?#]*@/g, "$1")
        .replace(QUERY_FALLBACK_PATTERN, `$1${REDACTED_SECRET}`),
    );
  }
}

export function redactMcpArgs(args: readonly string[]): string[] {
  const redacted: string[] = [];
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    const headerPrefix = argument.startsWith("--header=")
      ? "--header="
      : argument.startsWith("-H")
        ? "-H"
        : "";
    const header = /^\s*([^:=]+)\s*([:=])/.exec(
      argument.slice(headerPrefix.length),
    );
    if (header && isSensitiveHeaderName(header[1])) {
      redacted.push(
        `${headerPrefix}${header[1]}${header[2]}${REDACTED_SECRET}`,
      );
      continue;
    }
    const separator = argument.indexOf("=");
    if (
      separator > 0 &&
      (isSensitiveArgFlag(argument.slice(0, separator)) ||
        isSensitiveQueryKey(argument.slice(0, separator)))
    ) {
      redacted.push(`${argument.slice(0, separator)}=${REDACTED_SECRET}`);
      continue;
    }
    if (
      /^(?:sk[-_]|gh[pousr]_|github_pat_|eyJ)[A-Za-z0-9_.-]+/.test(argument) ||
      /\bBearer\s+/i.test(argument)
    ) {
      redacted.push(REDACTED_SECRET);
      continue;
    }
    if (argument.includes("://")) {
      redacted.push(redactMcpUrl(argument));
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

export function redactMcpHeaders(
  headers: Record<string, string>,
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(headers).map(([name, value]) => [
      name,
      isSensitiveHeaderName(name) ? REDACTED_SECRET : value,
    ]),
  );
}
