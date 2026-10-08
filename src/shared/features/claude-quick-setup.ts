import * as z from "zod/mini";
import { fileWriteTargetSchema, parseFileDisplayPath } from "./file-writes";

const target = z.enum(["claude_settings", "claude_mcp"]);
const displayPath = z.string().check(
  z.refine((value) => {
    try {
      parseFileDisplayPath(value);
      return true;
    } catch {
      return false;
    }
  }),
);
const identity = z
  .string()
  .check(
    z.regex(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    ),
  );
const applySchema = z.strictObject({ previewId: identity });
const previewSchema = z.strictObject({
  contractVersion: z.literal(1),
  previewId: identity,
  writeTargets: z.array(fileWriteTargetSchema).check(z.maxLength(2)),
  preservedPaths: z.array(displayPath).check(z.maxLength(2)),
});
const outcomeSchema = z.strictObject({
  contractVersion: z.literal(1),
  overall: z.enum(["applied", "partial", "stale", "rolledBack", "unknown"]),
  providerState: z.enum(["applied", "rolledBack", "unchanged", "unknown"]),
  files: z
    .array(
      z.strictObject({
        target,
        state: z.enum([
          "applied",
          "unchanged",
          "rolledBack",
          "conflict",
          "notAttempted",
          "unknown",
        ]),
      }),
    )
    .check(z.length(2)),
});

export type ClaudeQuickSetupPreview = z.infer<typeof previewSchema>;
export type ClaudeQuickSetupApplyRequest = z.infer<typeof applySchema>;
export type ClaudeQuickSetupOutcome = z.infer<typeof outcomeSchema>;

export function parseClaudeQuickSetupApplyRequest(
  value: unknown,
): ClaudeQuickSetupApplyRequest {
  return applySchema.parse(value);
}

export function parseClaudeQuickSetupPreview(
  value: unknown,
): ClaudeQuickSetupPreview {
  const result = previewSchema.parse(value);
  const paths = [
    ...result.writeTargets.map((item) => item.path),
    ...result.preservedPaths,
  ];
  if (paths.length !== 2 || new Set(paths).size !== 2) {
    throw new Error("无法确认 Claude 保存范围");
  }
  return result;
}

export function parseClaudeQuickSetupOutcome(
  value: unknown,
): ClaudeQuickSetupOutcome {
  const result = outcomeSchema.parse(value);
  const states = result.files.map((item) => item.state);
  if (
    new Set(result.files.map((item) => item.target)).size !== 2 ||
    (result.overall === "applied" &&
      (result.providerState !== "applied" ||
        states.some((state) => !["applied", "unchanged"].includes(state)))) ||
    (result.overall === "stale" &&
      (result.providerState !== "unchanged" ||
        states.some(
          (state) => !["notAttempted", "unchanged"].includes(state),
        ))) ||
    (result.overall === "rolledBack" &&
      (result.providerState !== "rolledBack" ||
        states.some(
          (state) =>
            !["rolledBack", "unchanged", "notAttempted"].includes(state),
        ))) ||
    (result.overall === "partial" &&
      (result.providerState !== "applied" ||
        states.every((state) => ["applied", "unchanged"].includes(state))))
  ) {
    throw new Error("无法确认 Claude 保存结果");
  }
  return result;
}
