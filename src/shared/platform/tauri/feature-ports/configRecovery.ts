import { invoke } from "@tauri-apps/api/core";

import {
  assertConfigRecoveryRequest,
  assertConfigRecoveryTargets,
  parseConfigRecoveryList,
  parseConfigRecoverySnapshot,
  type ConfigRecoveryPort,
} from "../../../features/config-recovery";

export function createConfigRecoveryPort(): ConfigRecoveryPort {
  return {
    list: async (targets) => {
      const checked = assertConfigRecoveryTargets(targets);
      return parseConfigRecoveryList(
        await invoke<unknown>("get_config_file_recoveries", {
          targets: checked,
        }),
        checked,
      );
    },
    restore: async (request) => {
      const checked = assertConfigRecoveryRequest(request);
      const result = parseConfigRecoverySnapshot(
        await invoke<unknown>("restore_config_file_recovery", {
          request: checked,
        }),
      );
      if (result.target !== checked.target || result.state === "available")
        throw new Error("无法确认文件是否已恢复，请刷新后检查");
      return result;
    },
  };
}
