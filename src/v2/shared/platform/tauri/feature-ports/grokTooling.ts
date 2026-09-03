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
      await invoke("run_tool_lifecycle_action", {
        tools: ["grok"],
        action: "install_official_npm",
      });
    },
    installNative: async () => {
      await invoke("run_tool_lifecycle_action", {
        tools: ["grok"],
        action: "install_native",
      });
    },
  };
}
