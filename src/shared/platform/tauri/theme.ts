import { invoke } from "@tauri-apps/api/core";
import type { ThemePreference } from "../../design-system/appearance";

export function setNativeWindowTheme(theme: ThemePreference): Promise<void> {
  return invoke<void>("set_window_theme", { theme });
}
