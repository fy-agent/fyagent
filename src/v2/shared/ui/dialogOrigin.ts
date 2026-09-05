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
  if (
    !box.width ||
    !box.height ||
    box.left < 0 ||
    box.top < 0 ||
    box.right > innerWidth ||
    box.bottom > innerHeight
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
      (box.top < bounds.top || box.bottom > bounds.bottom)
    )
      return neutral;
    if (
      /(auto|scroll|hidden|clip)/.test(style.overflowX) &&
      (box.left < bounds.left || box.right > bounds.right)
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
