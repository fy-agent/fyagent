import { fySpatialEasing, motionDuration } from "./motion";

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

export function themeRevealGeometry(
  source: HTMLElement,
  origin?: { x: number; y: number },
) {
  const rect = source.getBoundingClientRect();
  const x = Math.min(
    window.innerWidth,
    Math.max(0, origin?.x ?? rect.left + rect.width / 2),
  );
  const y = Math.min(
    window.innerHeight,
    Math.max(0, origin?.y ?? rect.top + rect.height / 2),
  );
  return {
    x,
    y,
    radius:
      Math.hypot(
        Math.max(x, window.innerWidth - x),
        Math.max(y, window.innerHeight - y),
      ) + 1,
  };
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
    request?.animation?.cancel();
    request?.transition?.skipTransition();
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
    const { x, y, radius } = themeRevealGeometry(source, origin);
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
          const animation = root.animate(
            {
              clipPath: [
                `circle(0px at ${x}px ${y}px)`,
                `circle(${radius}px at ${x}px ${y}px)`,
              ],
            },
            {
              duration,
              easing: fySpatialEasing,
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
