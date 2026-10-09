import * as z from "zod/mini";

const filenameSchema = z.string().check(
  z.minLength(4),
  z.maxLength(255),
  z.refine(
    (value) =>
      value === value.trim() &&
      value.endsWith(".db") &&
      !value.includes("..") &&
      !/[\\/:*?"<>|]/u.test(value) &&
      Array.from(value).every((character) => {
        const code = character.charCodeAt(0);
        return code > 31 && (code < 127 || code > 159);
      }),
  ),
);

const backupSchema = z.strictObject({
  filename: filenameSchema,
  sizeBytes: z
    .number()
    .check(z.refine((value) => Number.isSafeInteger(value) && value >= 0)),
  createdAt: z
    .string()
    .check(
      z.refine(
        (value) =>
          value === "" ||
          (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/u.test(
            value,
          ) &&
            Number.isFinite(Date.parse(value))),
      ),
    ),
});

export const DATABASE_RESTORE_PHASES = [
  "precheck",
  "candidate",
  "archive",
  "safetyBackup",
  "publish",
  "readback",
] as const;
export const DATABASE_RESTORE_RESULT_CODES = [
  "restored",
  "precheckFailed",
  "candidateFailed",
  "archiveFailed",
  "safetyBackupFailed",
  "publishFailed",
  "readbackFailed",
  "workerLost",
] as const;

const outcomeSchema = z
  .strictObject({
    contractVersion: z.literal(1),
    phase: z.enum(DATABASE_RESTORE_PHASES),
    publication: z.enum(["notCommitted", "committed", "unknown"]),
    safetyBackupFilename: z.nullable(filenameSchema),
    warnings: z.array(z.enum(["retentionFailed", "projectionFailed"])).check(
      z.maxLength(2),
      z.refine((warnings) => new Set(warnings).size === warnings.length),
    ),
    resultCode: z.enum(DATABASE_RESTORE_RESULT_CODES),
  })
  .check(
    z.refine((outcome) => {
      if (
        outcome.publication === "committed" &&
        outcome.safetyBackupFilename === null
      )
        return false;
      if (
        outcome.warnings.includes("projectionFailed") &&
        outcome.publication !== "committed"
      )
        return false;
      if (
        outcome.warnings.includes("retentionFailed") &&
        outcome.phase !== "readback"
      )
        return false;
      switch (outcome.resultCode) {
        case "restored":
        case "readbackFailed":
          return (
            outcome.phase === "readback" && outcome.publication === "committed"
          );
        case "precheckFailed":
          return (
            outcome.phase === "precheck" &&
            outcome.publication === "notCommitted"
          );
        case "candidateFailed":
          return (
            outcome.phase === "candidate" &&
            outcome.publication === "notCommitted"
          );
        case "archiveFailed":
          return (
            outcome.phase === "archive" &&
            outcome.publication === "notCommitted"
          );
        case "safetyBackupFailed":
          return (
            outcome.phase === "safetyBackup" &&
            outcome.publication === "notCommitted"
          );
        case "publishFailed":
          return (
            outcome.phase === "publish" &&
            outcome.publication !== "committed" &&
            (outcome.publication !== "unknown" ||
              outcome.safetyBackupFilename !== null)
          );
        case "workerLost":
          return (
            outcome.publication === "unknown" ||
            (outcome.publication === "committed" &&
              (outcome.phase === "publish" || outcome.phase === "readback"))
          );
      }
    }),
  );

export type DatabaseBackupEntry = z.infer<typeof backupSchema>;
export type DatabaseRestoreOutcome = z.infer<typeof outcomeSchema>;

const readabilitySchema = z.strictObject({
  contractVersion: z.literal(1),
  state: z.enum(["readable", "unavailable"]),
});
export type DatabaseRecoveryReadability = z.infer<typeof readabilitySchema>;

export interface DatabaseRecoveryPort {
  list(): Promise<DatabaseBackupEntry[]>;
  restore(filename: string): Promise<DatabaseRestoreOutcome>;
  checkReadability(): Promise<DatabaseRecoveryReadability>;
}

/** An IPC or decoding failure carries no evidence that publication did not happen. */
export class DatabaseRestoreUnconfirmedError extends Error {
  readonly publication = "unknown";

  constructor() {
    super("无法确认恢复结果，请保留现有数据库和备份，刷新检查。");
    this.name = "DatabaseRestoreUnconfirmedError";
  }
}

export function assertDatabaseBackupFilename(value: unknown): string {
  const result = filenameSchema.safeParse(value);
  if (!result.success) throw new Error("请选择本机受管的 .db 备份文件");
  return result.data;
}

export function parseDatabaseBackupList(value: unknown): DatabaseBackupEntry[] {
  const result = z.array(backupSchema).safeParse(value);
  if (
    !result.success ||
    new Set(result.data.map((entry) => entry.filename)).size !==
      result.data.length
  ) {
    throw new Error("无法读取本机备份列表");
  }
  return result.data;
}

export function parseDatabaseRestoreOutcome(
  value: unknown,
): DatabaseRestoreOutcome {
  const result = outcomeSchema.safeParse(value);
  if (!result.success) throw new DatabaseRestoreUnconfirmedError();
  return result.data;
}

export function parseDatabaseRecoveryReadability(
  value: unknown,
): DatabaseRecoveryReadability {
  const result = readabilitySchema.safeParse(value);
  if (!result.success)
    throw new Error("无法检查当前数据库，请保留现有数据库和备份。");
  return result.data;
}
