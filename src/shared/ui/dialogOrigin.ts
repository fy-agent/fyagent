/** Explicit, feature-owned trigger reference. Never a global last-click store. */
export interface DialogOriginRef {
  current: HTMLElement | null;
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
