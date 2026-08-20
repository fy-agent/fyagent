import { useSyncExternalStore } from "react";

const WIDE_FEATURE_LAYOUT = "(min-width: 1181px)";

function subscribe(onStoreChange: () => void): () => void {
  if (typeof window.matchMedia !== "function") return () => undefined;
  const query = window.matchMedia(WIDE_FEATURE_LAYOUT);
  query.addEventListener("change", onStoreChange);
  return () => query.removeEventListener("change", onStoreChange);
}

function getSnapshot(): boolean {
  return typeof window.matchMedia === "function"
    ? window.matchMedia(WIDE_FEATURE_LAYOUT).matches
    : false;
}

export function useWideFeatureLayout(): boolean {
  return useSyncExternalStore(subscribe, getSnapshot, () => false);
}
