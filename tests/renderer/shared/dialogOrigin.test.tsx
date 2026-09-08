import { afterEach, describe, expect, it, vi } from "vitest";

import {
  captureDialogOrigin,
  dialogOriginGeometry,
  type DialogOriginRef,
} from "@/shared/ui/dialogOrigin";

function measuredElement(box: DOMRect, tag = "button"): HTMLElement {
  const element = document.createElement(tag);
  vi.spyOn(element, "getBoundingClientRect").mockReturnValue(box);
  document.body.append(element);
  return element;
}

const destination = new DOMRect(200, 160, 480, 320);

afterEach(() => {
  document.body.replaceChildren();
});

describe("explicit dialog origin geometry", () => {
  it("captures only geometry and allowlisted non-resource material for a transient trigger", () => {
    const source = measuredElement(new DOMRect(600, 60, 100, 40));
    const anchor = measuredElement(new DOMRect(700, 40, 80, 32));
    source.textContent = "private-label-not-for-animation";
    source.style.backgroundImage = 'url("https://example.invalid/private.png")';
    const ref: DialogOriginRef = { current: null };
    captureDialogOrigin(ref, source, anchor);
    expect(ref.returnTarget).toBe(anchor);
    expect(ref.snapshot?.box).toEqual({
      left: 600,
      top: 60,
      width: 100,
      height: 40,
    });
    expect(ref.snapshot?.material.backgroundImage).toBe("none");
    expect(Object.keys(ref.snapshot!).sort()).toEqual([
      "box",
      "element",
      "material",
      "radius",
    ]);
    const { element, ...data } = ref.snapshot!;
    expect(element).toBe(source);
    expect(JSON.stringify(data)).not.toMatch(
      /private-label|private\.png|innerHTML/,
    );
    source.remove();
    expect(ref.snapshot?.box.width).toBe(100);
    captureDialogOrigin(ref, anchor);
    expect(ref.snapshot).toBeUndefined();
    expect(ref.returnTarget).toBeUndefined();
  });
  it("maps the material centre and dimensions to the real source without copying content", () => {
    const source = measuredElement(new DOMRect(800, 50, 100, 40));
    source.textContent = "Never copy source text";
    expect(dialogOriginGeometry(source, destination)).toEqual({
      x: 410,
      y: -250,
      scaleX: 100 / 480,
      scaleY: 40 / 320,
      sourced: true,
    });
    expect(
      Object.keys(dialogOriginGeometry(source, destination)).sort(),
    ).toEqual(["scaleX", "scaleY", "sourced", "x", "y"]);
  });

  it("remeasures the source on return rather than reusing an obsolete click rectangle", () => {
    const source = measuredElement(new DOMRect(600, 60, 100, 40));
    const before = dialogOriginGeometry(source, destination);
    vi.mocked(source.getBoundingClientRect).mockReturnValue(
      new DOMRect(700, 110, 100, 40),
    );
    const after = dialogOriginGeometry(source, destination);
    expect(after.x - before.x).toBe(100);
    expect(after.y - before.y).toBe(50);
  });

  it.each(["hidden", "inert"])(
    "does not return into a %s source tree",
    (attribute) => {
      const source = measuredElement(new DOMRect(600, 60, 100, 40));
      const parent = document.createElement("div");
      parent.setAttribute(attribute, "");
      document.body.append(parent);
      parent.append(source);
      expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
    },
  );

  it.each(["display: none", "visibility: hidden", "opacity: 0"])(
    "does not use an invisible source (%s)",
    (style) => {
      const source = measuredElement(new DOMRect(600, 60, 100, 40));
      source.setAttribute("style", style);
      expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
    },
  );

  it("falls back for a disconnected, offscreen or zero-size origin", () => {
    expect(dialogOriginGeometry(null, destination).sourced).toBe(false);
    const source = measuredElement(new DOMRect(-30, 60, 100, 40));
    expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
    vi.mocked(source.getBoundingClientRect).mockReturnValue(new DOMRect());
    expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
    source.remove();
    expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
  });

  it("rejects an origin scrolled out of its own container even inside the window", () => {
    const scroller = measuredElement(new DOMRect(500, 100, 240, 180), "div");
    scroller.style.overflowY = "auto";
    const source = measuredElement(new DOMRect(520, 70, 100, 40));
    scroller.append(source);
    expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
    vi.mocked(source.getBoundingClientRect).mockReturnValue(
      new DOMRect(520, 130, 100, 40),
    );
    expect(dialogOriginGeometry(source, destination).sourced).toBe(true);
  });

  it("accepts at most one device-pixel rounding edge, not actual clipped content", () => {
    const scroller = measuredElement(new DOMRect(500, 100, 240, 180), "div");
    scroller.style.overflowY = "hidden";
    const source = measuredElement(new DOMRect(520, 99.5, 100, 40));
    scroller.append(source);
    expect(dialogOriginGeometry(source, destination).sourced).toBe(true);
    vi.mocked(source.getBoundingClientRect).mockReturnValue(
      new DOMRect(520, 97, 100, 40),
    );
    expect(dialogOriginGeometry(source, destination).sourced).toBe(false);
  });
});
