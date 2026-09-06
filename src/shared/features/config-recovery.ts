import * as z from "zod/mini";

import { fileWriteTargetSchema } from "./file-writes";

export const CONFIG_RECOVERY_TARGETS = [
  "claude_settings",
  "claude_mcp",
  "codex_auth",
  "codex_config",
  "codex_catalog",
  "grok_config",
  "opencode_config",
  "opencode_auth",
] as const;
export type ConfigRecoveryTarget = (typeof CONFIG_RECOVERY_TARGETS)[number];

export const CONFIG_RECOVERY_TARGET_LABELS: Record<
  ConfigRecoveryTarget,
  string
> = {
  claude_settings: "Claude Code 配置",
  claude_mcp: "Claude Code MCP 配置",
  codex_auth: "Codex 登录凭证",
  codex_config: "Codex 模型与配置",
  codex_catalog: "Codex 模型目录",
  grok_config: "Grok Build 配置",
  opencode_config: "OpenCode 配置",
  opencode_auth: "OpenCode 登录凭证",
};

const receipt = z
  .string()
  .check(
    z.regex(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u,
    ),
  );
const targetsSchema = z.array(z.enum(CONFIG_RECOVERY_TARGETS)).check(
  z.minLength(1),
  z.maxLength(CONFIG_RECOVERY_TARGETS.length),
  z.refine((targets) => new Set(targets).size === targets.length),
);
const requestSchema = z.strictObject({
  target: z.enum(CONFIG_RECOVERY_TARGETS),
  receiptId: receipt,
});
const snapshotSchema = z
  .strictObject({
    contractVersion: z.literal(1),
    target: z.enum(CONFIG_RECOVERY_TARGETS),
    writeTarget: fileWriteTargetSchema,
    state: z.enum([
      "available",
      "none",
      "manual_backup",
      "conflict",
      "unavailable",
    ]),
    receiptId: z.nullable(receipt),
    restoresExistingFile: z.nullable(z.boolean()),
  })
  .check(
    z.refine((snapshot) =>
      snapshot.state === "available"
        ? snapshot.receiptId !== null && snapshot.restoresExistingFile !== null
        : snapshot.receiptId === null && snapshot.restoresExistingFile === null,
    ),
  );

export type ConfigRecoverySnapshot = z.infer<typeof snapshotSchema>;
export type ConfigRecoveryRequest = z.infer<typeof requestSchema>;

export interface ConfigRecoveryPort {
  list(
    targets: readonly ConfigRecoveryTarget[],
  ): Promise<ConfigRecoverySnapshot[]>;
  restore(request: ConfigRecoveryRequest): Promise<ConfigRecoverySnapshot>;
}

export function assertConfigRecoveryTargets(
  value: unknown,
): ConfigRecoveryTarget[] {
  const result = targetsSchema.safeParse(value);
  if (!result.success) throw new Error("文件恢复请求无效");
  return result.data;
}

export function assertConfigRecoveryRequest(
  value: unknown,
): ConfigRecoveryRequest {
  const result = requestSchema.safeParse(value);
  if (!result.success) throw new Error("文件恢复请求无效");
  return result.data;
}

export function parseConfigRecoverySnapshot(
  value: unknown,
): ConfigRecoverySnapshot {
  const result = snapshotSchema.safeParse(value);
  if (!result.success) throw new Error("无法确认文件备份状态");
  return result.data;
}

export function parseConfigRecoveryList(
  value: unknown,
  targets: readonly ConfigRecoveryTarget[],
): ConfigRecoverySnapshot[] {
  if (!Array.isArray(value) || value.length !== targets.length)
    throw new Error("无法确认文件备份状态");
  const result = value.map(parseConfigRecoverySnapshot);
  if (
    new Set(result.map((entry) => entry.target)).size !== targets.length ||
    result.some((entry) => !targets.includes(entry.target))
  )
    throw new Error("无法确认文件备份状态");
  return result;
}
