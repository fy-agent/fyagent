import { invoke } from "@tauri-apps/api/core";

import {
  assertDatabaseBackupFilename,
  DatabaseRestoreUnconfirmedError,
  parseDatabaseBackupList,
  parseDatabaseRecoveryReadability,
  parseDatabaseRestoreOutcome,
  type DatabaseRecoveryPort,
} from "../../../features/database-recovery";

export type DatabaseRecoveryInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

async function invokeDatabaseRecovery(
  command: string,
  args?: Record<string, unknown>,
): Promise<unknown> {
  switch (command) {
    case "list_db_backups":
      return invoke<unknown>("list_db_backups");
    case "restore_db_backup_outcome":
      return invoke<unknown>("restore_db_backup_outcome", args);
    case "check_db_recovery_readability":
      return invoke<unknown>("check_db_recovery_readability");
    default:
      throw new Error("本机恢复操作无效");
  }
}

export function createDatabaseRecoveryPort(
  invokeCommand: DatabaseRecoveryInvoke = invokeDatabaseRecovery,
): DatabaseRecoveryPort {
  return {
    checkReadability: async () => {
      try {
        return parseDatabaseRecoveryReadability(
          await invokeCommand("check_db_recovery_readability"),
        );
      } catch {
        throw new Error("无法检查当前数据库，请保留现有数据库和备份。");
      }
    },
    list: async () => {
      try {
        return parseDatabaseBackupList(await invokeCommand("list_db_backups"));
      } catch {
        throw new Error("无法读取本机备份列表，请刷新后检查。");
      }
    },
    restore: async (filename) => {
      const checked = assertDatabaseBackupFilename(filename);
      try {
        return parseDatabaseRestoreOutcome(
          await invokeCommand("restore_db_backup_outcome", {
            filename: checked,
          }),
        );
      } catch {
        throw new DatabaseRestoreUnconfirmedError();
      }
    },
  };
}
