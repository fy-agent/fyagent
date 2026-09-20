import { invoke } from "@tauri-apps/api/core";

import {
  parseGrokToolSnapshot,
  type GrokToolingPort,
} from "../../../features/grok-tooling";

export function createGrokToolingPort(): GrokToolingPort {
  return {
    getSnapshot: async () =>
      parseGrokToolSnapshot(
        await invoke<unknown>("get_tool_versions", { tools: ["grok"] }),
      ),
    installOfficialNpm: async () => {
      throw new Error(
        "Direct tool installation without preflight is forbidden",
      );
    },
    installNative: async () => {
      throw new Error(
        "Direct tool installation without preflight is forbidden",
      );
    },
  };
}
