const KNOWN_TYPE_ORDER = [
  "gpt",
  "claude",
  "gemini",
  "grok",
  "deepseek",
  "qwen",
  "kimi",
  "glm",
  "llama",
  "mistral",
  "doubao",
  "minimax",
  "command",
] as const;

const KNOWN_TYPE_INDEX = new Map<string, number>(
  KNOWN_TYPE_ORDER.map((type, index) => [type, index]),
);

const TYPE_MATCHERS: ReadonlyArray<{
  type: (typeof KNOWN_TYPE_ORDER)[number];
  pattern: RegExp;
}> = [
  { type: "gpt", pattern: /^(gpt|chatgpt|o[1-9](?:\b|[-._]))/i },
  { type: "claude", pattern: /^claude/i },
  { type: "gemini", pattern: /^gemini/i },
  { type: "grok", pattern: /^grok/i },
  { type: "deepseek", pattern: /^deepseek/i },
  { type: "qwen", pattern: /^qwen/i },
  { type: "kimi", pattern: /^(kimi|moonshot)/i },
  { type: "glm", pattern: /^(glm|chatglm)/i },
  { type: "llama", pattern: /^llama/i },
  { type: "mistral", pattern: /^(mistral|mixtral|codestral|pixtral)/i },
  { type: "doubao", pattern: /^(doubao|ep-)/i },
  { type: "minimax", pattern: /^minimax/i },
  { type: "command", pattern: /^command/i },
];

export type ModelIdGroup = {
  type: string;
  ids: string[];
};

function modelLeaf(modelId: string): string {
  const separator = modelId.lastIndexOf("/");
  return separator === -1 ? modelId : modelId.slice(separator + 1);
}

export function classifyModelType(modelId: string): string {
  const trimmed = modelId.trim();
  if (!trimmed) return "other";
  const leaf = modelLeaf(trimmed);
  for (const { type, pattern } of TYPE_MATCHERS) {
    if (pattern.test(leaf) || pattern.test(trimmed)) return type;
  }
  const token = leaf.match(/^[A-Za-z][A-Za-z0-9]*/u);
  return token ? token[0].toLocaleLowerCase("en-US") : "other";
}

function compareModelTypes(left: string, right: string): number {
  const leftIndex = KNOWN_TYPE_INDEX.get(left);
  const rightIndex = KNOWN_TYPE_INDEX.get(right);
  if (leftIndex !== undefined && rightIndex !== undefined) {
    return leftIndex - rightIndex;
  }
  if (leftIndex !== undefined) return -1;
  if (rightIndex !== undefined) return 1;
  if (left === "other") return 1;
  if (right === "other") return -1;
  return left.localeCompare(right, "en-US");
}

export function groupModelIds(ids: readonly string[]): ModelIdGroup[] {
  const groups = new Map<string, string[]>();
  const seen = new Set<string>();

  for (const raw of ids) {
    const id = raw.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    const type = classifyModelType(id);
    const current = groups.get(type);
    if (current) current.push(id);
    else groups.set(type, [id]);
  }

  return [...groups.entries()]
    .sort(([left], [right]) => compareModelTypes(left, right))
    .map(([type, groupedIds]) => ({ type, ids: groupedIds }));
}

export function addUniqueModelIds(
  current: readonly string[],
  incoming: readonly string[],
): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const raw of [...current, ...incoming]) {
    const id = raw.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    result.push(id);
  }
  return result;
}

export function splitWorkBuddyDraft(
  draftIds: readonly string[],
  fetchedSourceIds: ReadonlySet<string>,
): { selectedModelIds: string[]; manualModelIds: string[] } {
  const selectedModelIds: string[] = [];
  const manualModelIds: string[] = [];
  const seen = new Set<string>();

  for (const raw of draftIds) {
    const id = raw.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    if (fetchedSourceIds.has(id)) selectedModelIds.push(id);
    else manualModelIds.push(id);
  }

  return { selectedModelIds, manualModelIds };
}

export function nativeErrorCode(error: unknown): string | null {
  if (typeof error !== "object" || error === null || !("code" in error)) {
    return null;
  }
  return typeof error.code === "string" ? error.code : null;
}

export function filterModelIds(
  ids: readonly string[],
  query: string,
): string[] {
  const needle = query.trim().toLocaleLowerCase("en-US");
  if (!needle) return [...ids];
  return ids.filter((id) => id.toLocaleLowerCase("en-US").includes(needle));
}
