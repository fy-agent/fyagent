import { useEffect, useRef, useState } from "react";
import {
  applyTheme,
  parseThemePreference,
  persistTheme,
  readThemePreference,
  resolveTheme,
  THEME_STORAGE_KEY,
  type ThemePreference,
} from "../design-system/appearance";
import { synchronizeWindowTheme } from "../platform/theme";
import { ThemeReveal } from "../ui/ThemeReveal";
import { useMediaQuery } from "../ui/useMediaQuery";

/** One shell control owns this UI preference; no app-wide resource cache or
 * duplicate settings document. Only this control rerenders when colors change. */
export function useAppearance() {
  const [theme, setTheme] = useState(() => resolveTheme(readThemePreference()));
  const preference = useRef(readThemePreference());
  const systemDark = useMediaQuery("(prefers-color-scheme: dark)");
  const reducedMotion = useMediaQuery("(prefers-reduced-motion: reduce)");
  const forcedColors = useMediaQuery("(forced-colors: active)");
  const reveal = useRef<ThemeReveal>();
  reveal.current ??= new ThemeReveal();

  useEffect(() => {
    const controller = reveal.current!;
    const refresh = (next: ThemePreference) => {
      // An external preference supersedes a pending local capture. Do not
      // persist the superseded intent while processing a storage event.
      controller.dispose();
      preference.current = next;
      setTheme(applyTheme(next));
      void synchronizeWindowTheme(next);
    };
    const storageChange = (event: StorageEvent) => {
      if (event.key === THEME_STORAGE_KEY || event.key === null)
        refresh(parseThemePreference(event.newValue));
    };
    const visibility = () => {
      if (document.hidden) controller.settle();
    };
    refresh(preference.current);
    window.addEventListener("resize", controller.settle);
    document.addEventListener("visibilitychange", visibility);
    window.addEventListener("storage", storageChange);
    return () => {
      controller.dispose();
      window.removeEventListener("resize", controller.settle);
      document.removeEventListener("visibilitychange", visibility);
      window.removeEventListener("storage", storageChange);
    };
  }, []);

  useEffect(() => {
    if (preference.current === "system") {
      reveal.current?.settle();
      setTheme(applyTheme("system"));
    }
  }, [systemDark]);
  useEffect(() => {
    if (reducedMotion || forcedColors) reveal.current?.settle();
  }, [reducedMotion, forcedColors]);

  const toggle = (source: HTMLElement, origin?: { x: number; y: number }) => {
    const next = resolveTheme(preference.current) === "dark" ? "light" : "dark";
    preference.current = next;
    reveal.current!.run(
      () => {
        setTheme(applyTheme(next));
        persistTheme(next);
        // No IPC or asynchronous work participates in the snapshot commit.
        queueMicrotask(() => {
          void synchronizeWindowTheme(next);
        });
      },
      source,
      origin,
    );
  };
  return { theme, toggle };
}
