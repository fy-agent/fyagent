import { describe, expect, it } from "vitest";

import {
  assertDatabaseBackupFilename,
  DatabaseRestoreUnconfirmedError,
  parseDatabaseBackupList,
  parseDatabaseRecoveryReadability,
  parseDatabaseRestoreOutcome,
} from "@/shared/features/database-recovery";

const committed = {
  contractVersion: 1,
  phase: "readback",
  publication: "committed",
  safetyBackupFilename: "before-restore.db",
  warnings: [],
  resultCode: "restored",
};

describe("closed database recovery contract", () => {
  it.each(["readable", "unavailable"])(
    "accepts a native %s observation only",
    (state) => {
      expect(
        parseDatabaseRecoveryReadability({ contractVersion: 1, state }),
      ).toEqual({ contractVersion: 1, state });
    },
  );
  it.each([
    null,
    {},
    { contractVersion: 2, state: "readable" },
    { contractVersion: 1, state: "restored" },
    { contractVersion: 1, state: "readable", content: "SECRET-CANARY" },
  ])("rejects an invalid readability observation %j", (value) => {
    expect(() => parseDatabaseRecoveryReadability(value)).toThrow(
      "无法检查当前数据库",
    );
  });
  it.each(["before-restore.db", "本机备份.db", "daily-20261007.db"])(
    "accepts the native leaf filename %s",
    (filename) => {
      expect(assertDatabaseBackupFilename(filename)).toBe(filename);
    },
  );

  it.each([
    "../backup.db",
    "..\\backup.db",
    "C:\\backup.db",
    "/tmp/backup.db",
    "backup.sql",
    ".db",
    "hidden..db",
    " backup.db",
    "backup.db\n",
    "backup?.db",
    "backup\u0000.db",
    "backup\u007f.db",
    "backup\u0085.db",
    "https://local/backup.db",
    "x".repeat(256) + ".db",
  ])("rejects a path or malformed leaf %j", (filename) => {
    expect(() => assertDatabaseBackupFilename(filename)).toThrow();
  });

  it("preserves an unknown backup time without inventing one", () => {
    expect(
      parseDatabaseBackupList([
        { filename: "old.db", sizeBytes: 4096, createdAt: "" },
        {
          filename: "new.db",
          sizeBytes: 8192,
          createdAt: "2026-10-07T08:00:00+08:00",
        },
      ]),
    ).toHaveLength(2);
  });

  it.each([
    null,
    {},
    [{ filename: "x.db", sizeBytes: -1, createdAt: "" }],
    [{ filename: "x.db", sizeBytes: 1.5, createdAt: "" }],
    [
      {
        filename: "x.db",
        sizeBytes: Number.MAX_SAFE_INTEGER + 1,
        createdAt: "",
      },
    ],
    [{ filename: "x.db", sizeBytes: 1, createdAt: "not a date" }],
    [{ filename: "x.db", sizeBytes: 1, createdAt: "", path: "SECRET-CANARY" }],
    [
      { filename: "x.db", sizeBytes: 1, createdAt: "" },
      { filename: "x.db", sizeBytes: 2, createdAt: "" },
    ],
  ])("rejects an invalid list %j", (value) => {
    expect(() => parseDatabaseBackupList(value)).toThrow(
      "无法读取本机备份列表",
    );
  });

  it.each([
    { phase: "precheck", resultCode: "precheckFailed" },
    { phase: "candidate", resultCode: "candidateFailed" },
    { phase: "archive", resultCode: "archiveFailed" },
    { phase: "safetyBackup", resultCode: "safetyBackupFailed" },
    { phase: "publish", resultCode: "publishFailed" },
  ])("accepts a proven prepublication failure at $phase", (failure) => {
    expect(
      parseDatabaseRestoreOutcome({
        ...committed,
        ...failure,
        publication: "notCommitted",
        safetyBackupFilename: null,
      }).publication,
    ).toBe("notCommitted");
  });

  it.each(["restored", "readbackFailed"])(
    "keeps %s committed with the recovery point",
    (resultCode) => {
      expect(
        parseDatabaseRestoreOutcome({
          ...committed,
          resultCode,
          warnings: ["retentionFailed"],
        }),
      ).toMatchObject({
        publication: "committed",
        safetyBackupFilename: "before-restore.db",
        warnings: ["retentionFailed"],
      });
    },
  );

  it.each(["committed", "unknown"])(
    "keeps worker loss %s when that is native evidence",
    (publication) => {
      expect(
        parseDatabaseRestoreOutcome({
          ...committed,
          phase: "publish",
          publication,
          resultCode: "workerLost",
        }).publication,
      ).toBe(publication);
    },
  );

  it("preserves unknown publication failure with the exact native recovery point", () => {
    expect(
      parseDatabaseRestoreOutcome({
        ...committed,
        phase: "publish",
        publication: "unknown",
        resultCode: "publishFailed",
      }),
    ).toMatchObject({
      publication: "unknown",
      safetyBackupFilename: "before-restore.db",
    });
  });

  it("permits worker loss before a publication stage without fabricating commitment", () => {
    expect(
      parseDatabaseRestoreOutcome({
        ...committed,
        phase: "candidate",
        publication: "unknown",
        safetyBackupFilename: null,
        resultCode: "workerLost",
      }),
    ).toMatchObject({ publication: "unknown", safetyBackupFilename: null });
  });

  it.each([
    null,
    {},
    { ...committed, contractVersion: 2 },
    { ...committed, phase: "maintenance" },
    { ...committed, publication: "notCommitted" },
    { ...committed, resultCode: "failed" },
    { ...committed, safetyBackupFilename: null },
    { ...committed, safetyBackupFilename: "/secret/backup.db" },
    { ...committed, warnings: ["SECRET-CANARY"] },
    { ...committed, warnings: ["retentionFailed", "retentionFailed"] },
    { ...committed, secret: "SECRET-CANARY" },
    { ...committed, resultCode: "candidateFailed" },
    { ...committed, resultCode: "publishFailed" },
    {
      ...committed,
      phase: "publish",
      resultCode: "publishFailed",
      publication: "unknown",
      safetyBackupFilename: null,
    },
    { ...committed, phase: "archive", resultCode: "workerLost" },
    {
      ...committed,
      phase: "candidate",
      resultCode: "candidateFailed",
      publication: "notCommitted",
      warnings: ["retentionFailed"],
    },
    { ...committed, resultCode: "workerLost", publication: "notCommitted" },
  ])("does not convert an invalid outcome into notCommitted %j", (value) => {
    try {
      parseDatabaseRestoreOutcome(value);
      expect.fail("invalid outcome was accepted");
    } catch (error) {
      expect(error).toBeInstanceOf(DatabaseRestoreUnconfirmedError);
      expect(error).toMatchObject({ publication: "unknown" });
      expect(String(error)).not.toContain("SECRET-CANARY");
    }
  });
});
