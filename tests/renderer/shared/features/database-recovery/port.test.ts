import { describe, expect, it, vi } from "vitest";

import { DatabaseRestoreUnconfirmedError } from "@/shared/features/database-recovery";
import { createDatabaseRecoveryPort } from "@/shared/platform/tauri/feature-ports/databaseRecovery";

const { nativeInvoke } = vi.hoisted(() => ({ nativeInvoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: nativeInvoke }));

const outcome = {
  contractVersion: 1,
  phase: "readback",
  publication: "committed",
  safetyBackupFilename: "before-restore.db",
  warnings: ["retentionFailed"],
  resultCode: "readbackFailed",
};

describe("database recovery native adapter", () => {
  it("dispatches the three production commands using their literal native calls", async () => {
    nativeInvoke.mockReset();
    nativeInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(outcome)
      .mockResolvedValueOnce({ contractVersion: 1, state: "readable" });
    const port = createDatabaseRecoveryPort();
    await expect(port.list()).resolves.toEqual([]);
    await expect(port.restore("selected.db")).resolves.toMatchObject({
      publication: "committed",
    });
    await expect(port.checkReadability()).resolves.toEqual({
      contractVersion: 1,
      state: "readable",
    });
    expect(nativeInvoke.mock.calls).toEqual([
      ["list_db_backups"],
      ["restore_db_backup_outcome", { filename: "selected.db" }],
      ["check_db_recovery_readability"],
    ]);
  });

  it("checks current database readability using a read-only no-argument command", async () => {
    const invoke = vi
      .fn()
      .mockResolvedValue({ contractVersion: 1, state: "readable" });
    await expect(
      createDatabaseRecoveryPort(invoke).checkReadability(),
    ).resolves.toEqual({ contractVersion: 1, state: "readable" });
    expect(invoke).toHaveBeenCalledExactlyOnceWith(
      "check_db_recovery_readability",
    );
  });
  it("rejects guessed readability and sanitizes the response", async () => {
    const invoke = vi.fn().mockResolvedValue({
      contractVersion: 1,
      state: "readable",
      sql: "SECRET-CANARY",
    });
    await expect(
      createDatabaseRecoveryPort(invoke).checkReadability(),
    ).rejects.toThrow("无法检查当前数据库");
    await expect(
      createDatabaseRecoveryPort(invoke).checkReadability(),
    ).rejects.not.toThrow("CANARY");
  });
  it("lists local backups and restores only a leaf filename", async () => {
    const invoke = vi
      .fn()
      .mockResolvedValueOnce([
        { filename: "selected.db", sizeBytes: 4096, createdAt: "" },
      ])
      .mockResolvedValueOnce(outcome);
    const port = createDatabaseRecoveryPort(invoke);
    await expect(port.list()).resolves.toHaveLength(1);
    expect(invoke).toHaveBeenNthCalledWith(1, "list_db_backups");
    await expect(port.restore("selected.db")).resolves.toMatchObject({
      publication: "committed",
      safetyBackupFilename: "before-restore.db",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "restore_db_backup_outcome", {
      filename: "selected.db",
    });
  });

  it("rejects caller paths before dispatch", async () => {
    const invoke = vi.fn();
    await expect(
      createDatabaseRecoveryPort(invoke).restore("C:\\private\\selected.db"),
    ).rejects.toThrow("请选择本机受管");
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each(["SECRET-CANARY C:\\private\\db", new Error("SECRET-CANARY")])(
    "keeps transport rejection unknown and sanitizes %j",
    async (error) => {
      const invoke = vi.fn().mockRejectedValue(error);
      const result = createDatabaseRecoveryPort(invoke).restore("selected.db");
      await expect(result).rejects.toBeInstanceOf(
        DatabaseRestoreUnconfirmedError,
      );
      await expect(result).rejects.toMatchObject({ publication: "unknown" });
      await expect(result).rejects.not.toThrow("CANARY");
      expect(invoke).toHaveBeenCalledTimes(1);
    },
  );

  it("treats malformed native success as unknown without retrying the command", async () => {
    const invoke = vi
      .fn()
      .mockResolvedValue({ ...outcome, publication: "notCommitted" });
    await expect(
      createDatabaseRecoveryPort(invoke).restore("selected.db"),
    ).rejects.toMatchObject({ publication: "unknown" });
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("never exposes a raw list error", async () => {
    const invoke = vi.fn().mockRejectedValue("SECRET-CANARY");
    await expect(createDatabaseRecoveryPort(invoke).list()).rejects.toThrow(
      "无法读取本机备份列表",
    );
    await expect(createDatabaseRecoveryPort(invoke).list()).rejects.not.toThrow(
      "CANARY",
    );
  });
});
