import { useEffect } from "react";

import { usePersistentVisibility } from "../ui/PersistentSurface";
import { signalFrontendReady } from "./lifecycle";

async function decodeInitialArtwork(): Promise<void> {
  const images = Array.from(
    document.querySelectorAll<HTMLImageElement>("img[data-fy-startup-image]"),
  );
  await Promise.all(
    images.map(async (image) => {
      if (
        image.closest("[hidden], [inert]") ||
        image.loading === "lazy" ||
        typeof image.decode !== "function"
      )
        return;
      try {
        const source = new URL(image.currentSrc || image.src, document.baseURI);
        if (
          source.protocol !== "data:" &&
          source.origin !== window.location.origin
        )
          return;
        await image.decode();
      } catch {
        // A failed decorative asset must not hide an otherwise usable/error UI.
      }
    }),
  );
}

/** A committed usable/error surface, never the shell or a Suspense fallback. */
export function useFrontendReady(ready = true): void {
  const visible = usePersistentVisibility();
  useEffect(() => {
    if (!ready || !visible) return;
    // Hidden native WebViews may throttle animation frames. Commit is the
    // prerequisite; waiting for visibility/RAF here would deadlock reveal.
    let current = true;
    void decodeInitialArtwork()
      .then(() => (current ? signalFrontendReady() : undefined))
      .catch(() => {
        console.error("FyAgent could not signal frontend readiness.");
      });
    return () => {
      current = false;
    };
  }, [ready, visible]);
}
