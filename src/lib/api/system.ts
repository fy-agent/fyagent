import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";

export interface SkillsMigrationResult {
  count: number;
  error?: string;
}

export const systemApi = {
  async getVersion(): Promise<string> {
    return await getVersion();
  },

  async getMigrationResult(): Promise<boolean> {
    return await invoke("get_migration_result");
  },

  async getSkillsMigrationResult(): Promise<SkillsMigrationResult | null> {
    return await invoke("get_skills_migration_result");
  },

  async setWindowTheme(theme: "light" | "dark" | "system"): Promise<void> {
    await invoke("set_window_theme", { theme });
  },

  async exit(): Promise<void> {
    await invoke("exit_app");
  },
};
