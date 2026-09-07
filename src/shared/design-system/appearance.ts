export type Theme = "light" | "dark";
export type ThemePreference = Theme | "system";
export const THEME_STORAGE_KEY = "fyagent-theme";

export function parseThemePreference(value: unknown): ThemePreference {
  return value === "dark" || value === "system" ? value : "light";
}

export function readThemePreference(): ThemePreference {
  try {
    return parseThemePreference(localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return "light";
  }
}

export function resolveTheme(preference: ThemePreference): Theme {
  return preference === "system"
    ? window.matchMedia?.("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light"
    : preference;
}

export function applyTheme(preference: ThemePreference): Theme {
  const theme = resolveTheme(preference);
  document.documentElement.dataset.theme = theme;
  return theme;
}

export function persistTheme(preference: ThemePreference): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, preference);
  } catch {
    // A denied preference store must not disable appearance or application startup.
  }
}
