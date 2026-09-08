/** Explicit, feature-owned trigger reference. Never a global last-click store. */
export interface DialogOriginRef {
  current: HTMLElement | null;
  /** One-use opening geometry for an explicit transient or async-reflow action. */
  snapshot?: DialogOriginSnapshot;
  returnTarget?: HTMLElement | null;
}

export const dialogRadiusProperties = [
  "borderTopLeftRadius",
  "borderTopRightRadius",
  "borderBottomRightRadius",
  "borderBottomLeftRadius",
] as const;
const materialProperties = [
  "backgroundColor",
  "backgroundImage",
  "boxShadow",
  "borderTopColor",
  "borderRightColor",
  "borderBottomColor",
  "borderLeftColor",
  "borderTopWidth",
  "borderRightWidth",
  "borderBottomWidth",
  "borderLeftWidth",
  "borderTopStyle",
  "borderRightStyle",
  "borderBottomStyle",
  "borderLeftStyle",
] as const;
export type DialogRadius = Pick<
  CSSStyleDeclaration,
  (typeof dialogRadiusProperties)[number]
>;
export interface DialogOriginSnapshot {
  element: HTMLElement;
  box: Pick<DOMRect, "left" | "top" | "width" | "height">;
  radius: DialogRadius;
  material: Partial<Record<(typeof materialProperties)[number], string>>;
}

export function isDialogOriginSnapshotUsable(
  snapshot: DialogOriginSnapshot,
): boolean {
  const { left, top, width, height } = snapshot.box;
  const tolerance = 1 / Math.max(1, window.devicePixelRatio || 1);
  return (
    [left, top, width, height].every(Number.isFinite) &&
    width > 0 &&
    height > 0 &&
    left >= -tolerance &&
    top >= -tolerance &&
    left + width <= innerWidth + tolerance &&
    top + height <= innerHeight + tolerance
  );
}

/** Reuse the existing allowlist. Never read text, values, HTML or URL resources. */
export function readDialogMaterial(source: HTMLElement) {
  const style = getComputedStyle(source);
  return Object.fromEntries(
    materialProperties.map((key) => [
      key,
      /url\(/i.test(style[key]) ? "none" : style[key],
    ]),
  );
}

export function captureDialogOrigin(
  ref: DialogOriginRef,
  source: HTMLElement,
  returnTarget?: HTMLElement | null,
): void {
  ref.current = source;
  delete ref.snapshot;
  delete ref.returnTarget;
  if (!returnTarget) return;
  const rect = source.getBoundingClientRect();
  if (!dialogOriginGeometry(source, rect).sourced) return;
  const style = getComputedStyle(source);
  ref.returnTarget = returnTarget;
  ref.snapshot = {
    element: source,
    box: {
      left: rect.left,
      top: rect.top,
      width: rect.width,
      height: rect.height,
    },
    radius: {
      borderTopLeftRadius: style.borderTopLeftRadius,
      borderTopRightRadius: style.borderTopRightRadius,
      borderBottomRightRadius: style.borderBottomRightRadius,
      borderBottomLeftRadius: style.borderBottomLeftRadius,
    },
    material: readDialogMaterial(source),
  };
}

export function dialogOriginGeometry(
  source: HTMLElement | null,
  destination: DOMRect,
) {
  const neutral = { x: 0, y: 0, scaleX: 0.96, scaleY: 0.96, sourced: false };
  if (
    !source?.isConnected ||
    !destination.width ||
    !destination.height ||
    source.closest("[hidden], [inert]")
  )
    return neutral;
  const box = source.getBoundingClientRect();
  // Native focus/scroll and device-pixel rounding can leave a half-pixel edge
  // outside an overflow box. Do not discard a visibly intact source for that
  // quantization, but keep genuinely clipped/scrolled-away sources neutral.
  const edgeTolerance = 1 / Math.max(1, window.devicePixelRatio || 1);
  if (
    !box.width ||
    !box.height ||
    box.left < -edgeTolerance ||
    box.top < -edgeTolerance ||
    box.right > innerWidth + edgeTolerance ||
    box.bottom > innerHeight + edgeTolerance
  )
    return neutral;
  for (let node: HTMLElement | null = source; node; node = node.parentElement) {
    const style = getComputedStyle(node);
    if (
      style.visibility === "hidden" ||
      style.display === "none" ||
      style.opacity === "0"
    )
      return neutral;
    if (node === source) continue;
    const bounds = node.getBoundingClientRect();
    if (
      /(auto|scroll|hidden|clip)/.test(style.overflowY) &&
      (box.top < bounds.top - edgeTolerance ||
        box.bottom > bounds.bottom + edgeTolerance)
    )
      return neutral;
    if (
      /(auto|scroll|hidden|clip)/.test(style.overflowX) &&
      (box.left < bounds.left - edgeTolerance ||
        box.right > bounds.right + edgeTolerance)
    )
      return neutral;
  }
  return {
    x: box.left + box.width / 2 - destination.left - destination.width / 2,
    y: box.top + box.height / 2 - destination.top - destination.height / 2,
    scaleX: Math.max(0.02, Math.min(1, box.width / destination.width)),
    scaleY: Math.max(0.02, Math.min(1, box.height / destination.height)),
    sourced: true,
  };
}
