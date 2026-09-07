import { afterEach, describe, expect, it, vi } from "vitest";
import { ThemeReveal, themeRevealClipPath, themeRevealGeometry } from "@/shared/ui/ThemeReveal";
import {
  applyTheme,
  parseThemePreference,
  persistTheme,
  readThemePreference,
} from "@/shared/design-system/appearance";

function deferred() {
  let resolve!: () => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<void>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const controllers: ThemeReveal[] = [];
afterEach(() => {
  controllers.forEach((controller) => controller.dispose());
  controllers.length = 0;
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.style.removeProperty("--fy-motion-theme");
  localStorage.clear();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  Reflect.deleteProperty(document, "startViewTransition");
  Reflect.deleteProperty(document.documentElement, "animate");
});

function setup() {
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) => ({ media: query, matches: false })),
  );
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  document.documentElement.style.setProperty("--fy-motion-theme", ".56s");
  const source = document.createElement("button");
  source.getBoundingClientRect = () =>
    ({ left: 15, top: 20, width: 40, height: 40 }) as DOMRect;
  const cycles: Array<{
    update: () => void;
    ready: ReturnType<typeof deferred>;
    finished: ReturnType<typeof deferred>;
    skipTransition: ReturnType<typeof vi.fn>;
  }> = [];
  const start = vi.fn((update: () => void) => {
    const cycle = {
      update,
      ready: deferred(),
      finished: deferred(),
      skipTransition: vi.fn(),
    };
    cycles.push(cycle);
    return {
      ready: cycle.ready.promise,
      finished: cycle.finished.promise,
      updateCallbackDone: Promise.resolve(),
      skipTransition: cycle.skipTransition,
    };
  });
  Object.defineProperty(document, "startViewTransition", {
    configurable: true,
    value: start,
  });
  const animationDone = deferred();
  const cancel = vi.fn();
  const animation: {
    effect: { pseudoElement: string };
    playState: AnimationPlayState;
    finished: Promise<void>;
    cancel: ReturnType<typeof vi.fn>;
  } = {
    effect: { pseudoElement: "::view-transition-new(root)" },
    playState: "running",
    finished: Promise.resolve(),
    cancel,
  };
  animation.finished = animationDone.promise.then(() => {
    animation.playState = "finished";
  });
  const animate = vi.fn(() => animation);
  Object.defineProperty(document.documentElement, "animate", {
    configurable: true,
    value: animate,
  });
  const controller = new ThemeReveal();
  controllers.push(controller);
  return { controller, source, cycles, start, animate, cancel, animationDone };
}

describe("appearance preference and browser-owned reveal", () => {
  it.each([
    ["dark", "dark"],
    ["system", "system"],
    ["light", "light"],
    [null, "light"],
    ["invalid", "light"],
  ])("validates preference %s", (value, expected) => {
    expect(parseThemePreference(value)).toBe(expected);
  });
  it("keeps startup and switching usable with denied storage", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });
    expect(readThemePreference()).toBe("light");
    expect(() => persistTheme("dark")).not.toThrow();
    expect(applyTheme("dark")).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
  it("uses exact pointer origin or the keyboard control center and covers all corners", () => {
    const { source } = setup();
    expect(themeRevealGeometry(source)).toMatchObject({ x: 35, y: 40 });
    const result = themeRevealGeometry(source, { x: 18, y: 26 });
    expect(result).toMatchObject({ x: 18, y: 26 });
    expect(result.radius).toBeGreaterThan(
      Math.hypot(innerWidth - 18, innerHeight - 26),
    );
    const clip = themeRevealClipPath(
      result.x,
      result.y,
      result.radius,
      innerWidth,
      innerHeight,
    );
    expect(clip.start).toMatch(/^circle\(0% at /);
    expect(clip.xPercent).toBeCloseTo((18 / innerWidth) * 100);
    expect(clip.yPercent).toBeCloseTo((26 / innerHeight) * 100);
  });
  it("commits once and holds CSS suppression through the whole reveal", async () => {
    const { controller, source, cycles, animate, animationDone, cancel } = setup();
    const commit = vi.fn();
    controller.run(commit, source);
    cycles[0].update();
    cycles[0].update();
    cycles[0].ready.resolve();
    await Promise.resolve();
    expect(commit).toHaveBeenCalledTimes(1);
    expect(animate).toHaveBeenCalledWith(
      expect.objectContaining({
        clipPath: [
          expect.stringMatching(/^circle\(0% at /),
          expect.stringMatching(/^circle\([\d.]+% at /),
        ],
      }),
      expect.objectContaining({
        duration: 560,
        easing: "cubic-bezier(0.25,0.08,0.25,1)",
        pseudoElement: "::view-transition-new(root)",
      }),
    );
    expect(document.documentElement.dataset.themeReveal).toBe("active");
    animationDone.resolve();
    await Promise.resolve();
    await Promise.resolve();
    expect(document.documentElement.dataset.themeReveal).toBeUndefined();
    expect(cancel).toHaveBeenCalledTimes(1);
  });
  it("rejects stale update/ready/finished callbacks after rapid reversal", async () => {
    const { controller, source, cycles, animate } = setup();
    const old = vi.fn();
    const latest = vi.fn();
    controller.run(old, source);
    controller.run(latest, source);
    cycles[0].update();
    cycles[0].ready.resolve();
    cycles[0].finished.resolve();
    await Promise.resolve();
    expect(old).not.toHaveBeenCalled();
    expect(animate).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.themeReveal).toBe("active");
    cycles[1].update();
    cycles[1].ready.resolve();
    await Promise.resolve();
    expect(latest).toHaveBeenCalledTimes(1);
    expect(animate).toHaveBeenCalledTimes(1);
  });
  it("settles a pending capture exactly once on interruption", async () => {
    const { controller, source, cycles } = setup();
    const commit = vi.fn();
    controller.run(commit, source);
    controller.settle();
    cycles[0].update();
    cycles[0].ready.reject(new Error("skipped"));
    await Promise.resolve();
    expect(commit).toHaveBeenCalledTimes(1);
    expect(document.documentElement.dataset.themeReveal).toBeUndefined();
  });
  it("falls back if capture throws or the browser rejects animation readiness", async () => {
    const { controller, source, start, cycles } = setup();
    const commit = vi.fn();
    controller.run(commit, source);
    cycles[0].ready.reject(new Error("unsupported"));
    await Promise.resolve();
    expect(commit).toHaveBeenCalledTimes(1);
    start.mockImplementation(() => {
      throw new Error("unsupported");
    });
    controller.run(commit, source);
    expect(commit).toHaveBeenCalledTimes(2);
  });
  it("does not snapshot a dialog, hidden page, reduced motion or missing API", () => {
    const { controller, source, start } = setup();
    const commit = vi.fn();
    const overlay = document.createElement("div");
    overlay.className = "fy-control-dialog-overlay";
    document.body.append(overlay);
    controller.run(commit, source);
    overlay.remove();
    vi.spyOn(document, "hidden", "get").mockReturnValue(true);
    controller.run(commit, source);
    vi.spyOn(document, "hidden", "get").mockReturnValue(false);
    vi.stubGlobal(
      "matchMedia",
      vi.fn((query: string) => ({ matches: query.includes("reduced-motion") })),
    );
    controller.run(commit, source);
    vi.stubGlobal(
      "matchMedia",
      vi.fn((query: string) => ({ matches: query.includes("forced-colors") })),
    );
    controller.run(commit, source);
    Reflect.deleteProperty(document, "startViewTransition");
    controller.run(commit, source);
    expect(start).not.toHaveBeenCalled();
    expect(commit).toHaveBeenCalledTimes(5);
  });
});
