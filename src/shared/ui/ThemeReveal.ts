import { fyThemeRevealEasing, motionDuration } from "./motion";

type NativeTransition = {
  ready: Promise<void>;
  finished: Promise<void>;
  updateCallbackDone: Promise<void>;
  skipTransition(): void;
};
type RevealRequest = {
  commit(): void;
  transition?: NativeTransition;
  animation?: Animation;
};

function themeRevealReferenceBox() {
  const root = document.documentElement;
  const viewport = window.visualViewport;
  return {
    width: Math.max(
      window.innerWidth || 0,
      root.clientWidth || 0,
      root.getBoundingClientRect().width || 0,
      viewport?.width ?? 0,
    ),
    height: Math.max(
      window.innerHeight || 0,
      root.clientHeight || 0,
      root.getBoundingClientRect().height || 0,
      viewport?.height ?? 0,
    ),
  };
}

function themeRevealRadiusPadding() {
  return Math.max(24, (window.devicePixelRatio || 1) * 8);
}

export function themeRevealGeometry(
  source: HTMLElement,
  origin?: { x: number; y: number },
) {
  const rect = source.getBoundingClientRect();
  const { width, height } = themeRevealReferenceBox();
  const x = Math.min(
    width,
    Math.max(0, origin?.x ?? rect.left + rect.width / 2),
  );
  const y = Math.min(
    height,
    Math.max(0, origin?.y ?? rect.top + rect.height / 2),
  );
  return {
    x,
    y,
    radius:
      Math.hypot(Math.max(x, width - x), Math.max(y, height - y)) +
      themeRevealRadiusPadding(),
  };
}

export function themeRevealClipPath(
  x: number,
  y: number,
  radius: number,
  width: number,
  height: number,
) {
  const safeWidth = width > 0 ? width : 1;
  const safeHeight = height > 0 ? height : 1;
  const xPercent = (x / safeWidth) * 100;
  const yPercent = (y / safeHeight) * 100;
  const radiusPercent =
    (radius / (Math.hypot(safeWidth, safeHeight) / Math.SQRT2)) * 100;
  return {
    xPercent,
    yPercent,
    radiusPercent,
    start: `circle(0% at ${xPercent}% ${yPercent}%)`,
    end: `circle(${radiusPercent}% at ${xPercent}% ${yPercent}%)`,
  };
}

function revealClipAnimations(): Animation[] {
  if (typeof document.getAnimations !== "function") return [];
  return document.getAnimations().filter(
    (animation) =>
      (animation.effect as KeyframeEffect | null)?.pseudoElement ===
      "::view-transition-new(root)",
  );
}

function releaseRevealClips(): void {
  for (const animation of revealClipAnimations()) animation.cancel();
}

/** Only presentation: preference, routing and native synchronization have other owners.
 * Browser-owned ephemeral snapshots are never read, cloned or exported by the app. */
export class ThemeReveal {
  private active: RevealRequest | undefined;

  settle = () => this.cancel(true);
  dispose = () => this.cancel(false);

  private cancel(commit: boolean): void {
    const request = this.active;
    if (commit) request?.commit();
    this.active = undefined;
    request?.transition?.skipTransition();
    request?.animation?.cancel();
    releaseRevealClips();
    delete document.documentElement.dataset.themeReveal;
  }

  run(
    commit: () => void,
    source: HTMLElement,
    origin?: { x: number; y: number },
  ): void {
    this.cancel(false);
    const doc = document as Document & {
      startViewTransition?: (update: () => void) => NativeTransition;
    };
    const root = document.documentElement;
    const duration = motionDuration("theme") * 1000;
    if (
      !doc.startViewTransition ||
      !root.animate ||
      !duration ||
      document.hidden ||
      typeof window.matchMedia !== "function" ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches ||
      window.matchMedia("(forced-colors: active)").matches ||
      document.querySelector(".fy-control-dialog-overlay")
    ) {
      commit();
      return;
    }
    let committed = false;
    const request: RevealRequest = {
      commit: () => {
        if (this.active !== request || committed) return;
        committed = true;
        commit();
      },
    };
    this.active = request;
    root.dataset.themeReveal = "active";
    try {
      const transition = doc.startViewTransition(request.commit);
      request.transition = transition;
      const finish = () => {
        if (this.active === request) this.settle();
      };
      void transition.updateCallbackDone.catch(finish);
      void transition.ready.then(() => {
        if (this.active !== request) return;
        try {
          const { x, y, radius } = themeRevealGeometry(source, origin);
          const { width, height } = themeRevealReferenceBox();
          const clip = themeRevealClipPath(x, y, radius, width, height);
          const animation = root.animate(
            {
              clipPath: [clip.start, clip.end],
            },
            {
              duration,
              easing: fyThemeRevealEasing,
              fill: "both",
              pseudoElement: "::view-transition-new(root)",
            },
          );
          request.animation = animation;
          if (
            (animation.effect as KeyframeEffect | null)?.pseudoElement !==
            "::view-transition-new(root)"
          ) {
            finish();
            return;
          }
          void animation.finished.then(finish, finish);
        } catch {
          finish();
        }
      }, finish);
      void transition.finished.then(finish, finish);
    } catch {
      this.settle();
    }
  }
}
