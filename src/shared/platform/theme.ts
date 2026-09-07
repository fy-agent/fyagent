import type { ThemePreference } from "../design-system/appearance";
import { detectRuntime } from "./runtime";
import { setNativeWindowTheme } from "./tauri/theme";

let pending: ThemePreference | undefined;
let applying = false;

/** Serialize the existing native command and coalesce superseded preferences.
 * Rendering never awaits IPC; an older native completion cannot be the last write. */
export async function synchronizeWindowTheme(
  theme: ThemePreference,
): Promise<void> {
  if (!detectRuntime().isNative) return;
  pending = theme;
  if (applying) return;
  applying = true;
  try {
    while (pending !== undefined) {
      const next = pending;
      pending = undefined;
      try {
        await setNativeWindowTheme(next);
      } catch {
        console.warn("Native appearance synchronization unavailable.");
      }
    }
  } finally {
    applying = false;
  }
}
