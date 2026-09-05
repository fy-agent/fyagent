import { afterEach, expect, it, vi } from "vitest";
import {
  runDialogPresentation,
  settleDialogPlanes,
  type DialogPlanes,
} from "@/v2/shared/ui/dialogPresentation";

const originalAnimate = Object.getOwnPropertyDescriptor(
  Element.prototype,
  "animate",
);
afterEach(() => {
  if (originalAnimate)
    Object.defineProperty(Element.prototype, "animate", originalAnimate);
  else Reflect.deleteProperty(Element.prototype, "animate");
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

function fixture(failAt?: number) {
  const calls: {
    target: Element;
    frames: Keyframe[];
    options: KeyframeAnimationOptions;
    cancel: ReturnType<typeof vi.fn>;
    complete: () => void;
  }[] = [];
  const animate = vi.fn(function (
    this: Element,
    frames: Keyframe[],
    options: KeyframeAnimationOptions,
  ) {
    if (calls.length === failAt) throw new Error("Native animation rejected");
    let complete!: () => void;
    let reject!: (error: Error) => void;
    const finished = new Promise<void>((resolve, fail) => {
      complete = resolve;
      reject = fail;
    });
    const cancel = vi.fn(() =>
      reject(new DOMException("Cancelled", "AbortError")),
    );
    calls.push({ target: this, frames, options, cancel, complete });
    return { finished, cancel };
  });
  Object.defineProperty(Element.prototype, "animate", {
    configurable: true,
    value: animate,
  });
  const windowNode = document.createElement("div");
  vi.spyOn(windowNode, "getBoundingClientRect").mockReturnValue(
    new DOMRect(200, 150, 500, 300),
  );
  document.body.append(windowNode);
  const source = document.createElement("button");
  source.textContent = "Do not copy identity";
  source.style.backgroundImage =
    'url("https://not-requested.invalid/image.png")';
  vi.spyOn(source, "getBoundingClientRect").mockReturnValue(
    new DOMRect(800, 50, 100, 36),
  );
  document.body.append(source);
  const planes: DialogPlanes = {
    material: document.createElement("div"),
    sourceMaterial: document.createElement("div"),
    targetMaterial: document.createElement("div"),
    foreground: document.createElement("div"),
    overlay: document.createElement("div"),
  };
  Object.values(planes).forEach((node) => windowNode.append(node));
  return { planes, source, windowNode, calls, animate };
}

it("preserves the 420ms geometry and 252/168ms content overlap without copying DOM or resources", async () => {
  const f = fixture();
  const run = runDialogPresentation({
    ...f,
    entering: true,
    first: true,
    duration: 420,
  });
  expect(f.calls[0].options).toMatchObject({
    delay: 80,
    duration: 340,
    easing: "cubic-bezier(0.32,0.72,0,1)",
  });
  const content = f.calls.find((call) => call.target === f.planes.foreground)!;
  expect(content.options.delay).toBe(252);
  expect(content.options.duration).toBeCloseTo(168);
  expect(f.planes.sourceMaterial.textContent).toBe("");
  expect(f.planes.sourceMaterial.childNodes).toHaveLength(0);
  expect(f.planes.sourceMaterial.style.backgroundImage).toBe("none");
  f.calls.forEach((call) => call.complete());
  await expect(run.finished).resolves.toBe(true);
  run.cancel(false);
  settleDialogPlanes(f.planes);
  expect(f.planes.material.style.width).toBe("");
  expect(f.planes.foreground.style.opacity).toBe("1");
});

it("cancels every already-started native track if a later layer fails", async () => {
  const f = fixture(2);
  expect(() =>
    runDialogPresentation({ ...f, entering: true, first: true, duration: 420 }),
  ).toThrow("Native animation rejected");
  expect(f.calls).toHaveLength(2);
  f.calls.forEach((call) => expect(call.cancel).toHaveBeenCalledTimes(1));
  await Promise.resolve(); // handled rejection must not escape after this test
});

it("treats cancellation as incomplete and freezes current values for reversal", async () => {
  const f = fixture();
  const run = runDialogPresentation({
    ...f,
    entering: true,
    first: true,
    duration: 420,
  });
  f.planes.material.style.width = "240px";
  f.planes.foreground.style.opacity = ".4";
  run.cancel();
  run.cancel();
  await expect(run.finished).resolves.toBe(false);
  f.calls.forEach((call) => expect(call.cancel).toHaveBeenCalledTimes(1));
  expect(f.planes.material.style.width).toBe("240px");
  expect(f.planes.foreground.style.opacity).toBe("0.4");
});

it("caps opt-in content exit at 80ms even when a caller retunes shell duration", async () => {
  const f = fixture();
  const run = runDialogPresentation({
    ...f,
    entering: false,
    first: false,
    duration: 900,
  });
  expect(
    f.calls.find((call) => call.target === f.planes.foreground)?.options
      .duration,
  ).toBe(80);
  run.cancel(false);
  await expect(run.finished).resolves.toBe(false);
});
