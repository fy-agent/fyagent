import {
  applyTheme,
  readThemePreference,
} from "../shared/design-system/appearance";

/** Runs before route loading/root rendering, including the startup error branch. */
export function initializeAppearance(): void {
  applyTheme(readThemePreference());
}
