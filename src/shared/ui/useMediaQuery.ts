import { useCallback, useSyncExternalStore } from "react";

/** Native media subscription shared by layout and motion; no resize polling. */
export function useMediaQuery(query: string, fallback = false): boolean {
  const subscribe = useCallback(
    (notify: () => void) => {
      if (
        typeof window === "undefined" ||
        typeof window.matchMedia !== "function"
      )
        return () => undefined;
      const media = window.matchMedia(query);
      if (typeof media.addEventListener === "function") {
        media.addEventListener("change", notify);
        return () => media.removeEventListener("change", notify);
      }
      media.addListener(notify);
      return () => media.removeListener(notify);
    },
    [query],
  );
  const snapshot = useCallback(
    () =>
      typeof window !== "undefined" && typeof window.matchMedia === "function"
        ? window.matchMedia(query).matches
        : fallback,
    [query, fallback],
  );
  return useSyncExternalStore(subscribe, snapshot, () => fallback);
}
