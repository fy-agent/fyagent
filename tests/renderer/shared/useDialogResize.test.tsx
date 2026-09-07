import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useDialogResize } from "@/shared/ui/useDialogResize";
import { runDialogResize } from "@/shared/ui/dialogPresentation";

vi.mock("@/shared/ui/motion", () => ({ motionDuration: () => 0.32 }));
vi.mock("@/shared/ui/dialogPresentation", () => ({
  runDialogResize: vi.fn(() => ({
    cancel: vi.fn(),
    finished: new Promise<boolean>(() => {}),
  })),
}));

function setup(from: DOMRect, target = from) {
  const root = document.createElement("div");
  const rootBox = vi
    .spyOn(root, "getBoundingClientRect")
    .mockReturnValue(target);
  const originSettler = { current: vi.fn() };
  const refs = {
    rootRef: { current: root },
    bodyRef: { current: document.createElement("div") },
    lastBoxRef: { current: from },
    originSettler,
    originRetargetRef: { current: vi.fn() },
  };
  const view = renderHook(
    ({ key }) =>
      useDialogResize({
        ...refs,
        present: true,
        settled: true,
        reduce: false,
        nativeAnimation: true,
        mountVersion: 1,
        presentationKey: key,
      }),
    { initialProps: { key: "first" } },
  );
  return { ...view, rootBox, refs };
}

describe("dialog intrinsic resize admission", () => {
  it("settles a CSS viewport width clamp without writing the old width back from an observer", () => {
    const { refs, unmount } = setup(
      new DOMRect(0, 0, 900, 756),
      new DOMRect(0, 0, 868, 756),
    );
    expect(runDialogResize).not.toHaveBeenCalled();
    expect(refs.originSettler.current).toHaveBeenCalledOnce();
    expect(refs.lastBoxRef.current.width).toBe(868);
    unmount();
  });
  it("still animates an intrinsic content-height change in the same session", () => {
    const { unmount } = setup(
      new DOMRect(0, 0, 480, 320),
      new DOMRect(0, 0, 480, 574),
    );
    expect(runDialogResize).toHaveBeenCalledWith(
      expect.objectContaining({
        from: expect.objectContaining({ width: 480, height: 320 }),
        target: expect.objectContaining({ width: 480, height: 574 }),
        duration: 320,
      }),
    );
    unmount();
  });
  it("allows width changes explicitly owned by a new size or presentation stage", () => {
    const { rerender, rootBox, unmount } = setup(new DOMRect(0, 0, 480, 320));
    rootBox.mockReturnValue(new DOMRect(0, 0, 720, 574));
    rerender({ key: "next-size" });
    expect(runDialogResize).toHaveBeenCalledWith(
      expect.objectContaining({
        target: expect.objectContaining({ width: 720, height: 574 }),
        duration: 320,
      }),
    );
    unmount();
  });
});
