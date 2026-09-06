import {
  dialogOriginGeometry,
  isDialogOriginSnapshotUsable,
  readDialogMaterial,
  type DialogOriginSnapshot,
  type DialogRadius,
} from "./dialogOrigin";
import { fySpatialEasing } from "./motion";

export interface DialogPlanes {
  material: HTMLElement;
  sourceMaterial: HTMLElement;
  targetMaterial: HTMLElement;
  foreground: HTMLElement;
  overlay: HTMLElement;
}

// These are presentation tracks, not business timers. Browser interpolation
// owns every frame. Fractions retain choreography when a duration is retuned.
export const presentationTracks = {
  pressLead: 80 / 420,
  contentEnterAt: 252 / 420,
  contentExit: 80 / 360,
  returnAt: 16 / 360,
  sourceReturnAt: 270 / 360,
} as const;

const geometryProperties = [
  "left",
  "top",
  "width",
  "height",
  "borderTopLeftRadius",
  "borderTopRightRadius",
  "borderBottomRightRadius",
  "borderBottomLeftRadius",
] as const;

/** Only allowlisted material strings are read. No source content, form value,
 * label, arbitrary attribute, clone, screenshot or URL resource is retained. */
function copyMaterial(source: HTMLElement, target: HTMLElement): void {
  Object.assign(target.style, readDialogMaterial(source));
}

function rectFrame(
  left: number,
  top: number,
  width: number,
  height: number,
  style: DialogRadius,
): Keyframe {
  return {
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
    height: `${height}px`,
    borderTopLeftRadius: style.borderTopLeftRadius,
    borderTopRightRadius: style.borderTopRightRadius,
    borderBottomRightRadius: style.borderBottomRightRadius,
    borderBottomLeftRadius: style.borderBottomLeftRadius,
  };
}

/** One isolated material plane morphs its real box/corners. The actual Radix
 * window and form keep final layout; there is no scale projection or DOM copy. */
export function runDialogPresentation({
  planes,
  source,
  windowNode,
  entering,
  first,
  duration,
  capturedOrigin,
}: {
  planes: DialogPlanes;
  source: HTMLElement | null;
  windowNode: HTMLElement;
  entering: boolean;
  first: boolean;
  duration: number; // milliseconds at the native animation boundary
  capturedOrigin?: DialogOriginSnapshot;
}) {
  const { material, sourceMaterial, targetMaterial, foreground, overlay } =
    planes;
  const box = windowNode.getBoundingClientRect();
  let destination = box;
  const capture =
    entering &&
    first &&
    capturedOrigin?.element === source &&
    isDialogOriginSnapshotUsable(capturedOrigin)
      ? capturedOrigin
      : undefined;
  const origin = {
    ...dialogOriginGeometry(source, box),
    ...(capture ? { sourced: true } : {}),
  };
  const originBox =
    capture?.box ?? (origin.sourced ? source?.getBoundingClientRect() : null);
  const style = getComputedStyle(windowNode);
  const resting = rectFrame(-1, -1, box.width, box.height, style);
  const sourceFrame =
    originBox && source
      ? rectFrame(
          originBox.left - box.left - 1,
          originBox.top - box.top - 1,
          originBox.width,
          originBox.height,
          capture?.radius ?? getComputedStyle(source),
        )
      : rectFrame(
          box.width * 0.02 - 1,
          box.height * 0.02 - 1,
          box.width * 0.96,
          box.height * 0.96,
          style,
        );
  windowNode.dataset.motionOrigin = origin.sourced ? "trigger" : "neutral";
  if (capture) Object.assign(sourceMaterial.style, capture.material);
  else if (origin.sourced && source) copyMaterial(source, sourceMaterial);
  const animations: Animation[] = [];
  let cancelled = false;
  const play = (
    node: HTMLElement,
    frames: Keyframe[],
    options: KeyframeAnimationOptions,
  ) => {
    const animation = node.animate(frames, { fill: "both", ...options });
    // Register before another track can throw; cancellation never leaves an
    // earlier animation writing into a failed or replaced presentation.
    animations.push(animation);
    return animation;
  };
  const snapshot = (
    node: HTMLElement,
    properties: readonly ((typeof geometryProperties)[number] | "opacity")[],
  ) => {
    const computed = getComputedStyle(node);
    return properties.map((key) => [key, computed[key]] as const);
  };
  const cancel = (freeze = true) => {
    if (cancelled) return;
    cancelled = true;
    const values = freeze
      ? [
          [
            material,
            snapshot(material, [...geometryProperties, "opacity"]),
          ] as const,
          ...[sourceMaterial, targetMaterial, foreground, overlay].map(
            (node) => [node, snapshot(node, ["opacity"])] as const,
          ),
        ]
      : [];
    animations.forEach((animation) => animation.cancel());
    for (const [node, entries] of values)
      for (const [key, value] of entries) node.style[key] = value;
  };
  const done = (animation: Animation) =>
    animation.finished.then(
      () => true,
      () => false,
    );
  try {
    if (first) {
      Object.assign(material.style, sourceFrame);
      material.style.opacity = "0";
      sourceMaterial.style.opacity = origin.sourced ? "1" : "0";
      targetMaterial.style.opacity = "0";
      foreground.style.opacity = "0";
      overlay.style.opacity = "0";
    }
    const current = Object.fromEntries(snapshot(material, geometryProperties));
    const lead =
      entering && first && origin.sourced
        ? Math.min(100, duration * presentationTracks.pressLead)
        : 0;
    const sourceOpacity = getComputedStyle(sourceMaterial).opacity;
    const geometry = play(
      material,
      [current, entering ? resting : sourceFrame],
      {
        duration:
          duration - (entering ? lead : duration * presentationTracks.returnAt),
        delay: entering ? lead : duration * presentationTracks.returnAt,
        easing: fySpatialEasing,
      },
    );
    play(
      material,
      entering
        ? [
            { opacity: first ? 0 : getComputedStyle(material).opacity },
            { opacity: 1 },
          ]
        : [
            { opacity: getComputedStyle(material).opacity, offset: 0 },
            {
              opacity: getComputedStyle(material).opacity,
              offset: presentationTracks.sourceReturnAt,
            },
            { opacity: 0, offset: 1 },
          ],
      entering
        ? {
            duration: first ? duration * 0.1 : duration * 0.25,
            delay: lead,
            easing: "linear",
          }
        : { duration, easing: "linear" },
    );
    play(
      sourceMaterial,
      entering
        ? [
            { opacity: sourceOpacity, offset: 0 },
            { opacity: sourceOpacity, offset: 0.14 },
            { opacity: 0, offset: 0.42 },
            { opacity: 0, offset: 1 },
          ]
        : [
            { opacity: sourceOpacity, offset: 0 },
            { opacity: sourceOpacity, offset: 0.18 },
            { opacity: origin.sourced ? 1 : 0, offset: 0.88 },
            { opacity: origin.sourced ? 1 : 0, offset: 1 },
          ],
      { duration, easing: "linear" },
    );
    play(
      targetMaterial,
      entering
        ? [
            { opacity: getComputedStyle(targetMaterial).opacity, offset: 0 },
            { opacity: getComputedStyle(targetMaterial).opacity, offset: 0.1 },
            { opacity: 1, offset: 0.52 },
            { opacity: 1, offset: 1 },
          ]
        : [
            { opacity: getComputedStyle(targetMaterial).opacity, offset: 0 },
            { opacity: getComputedStyle(targetMaterial).opacity, offset: 0.12 },
            { opacity: 0, offset: 0.78 },
            { opacity: 0, offset: 1 },
          ],
      { duration, easing: "linear" },
    );
    const content = play(
      foreground,
      [
        { opacity: getComputedStyle(foreground).opacity },
        { opacity: entering ? 1 : 0 },
      ],
      {
        duration: entering
          ? duration * (1 - presentationTracks.contentEnterAt)
          : Math.min(80, duration * presentationTracks.contentExit),
        delay:
          entering && first ? duration * presentationTracks.contentEnterAt : 0,
        easing: entering ? "cubic-bezier(0,0,.2,1)" : "cubic-bezier(.4,0,1,1)",
      },
    );
    play(
      overlay,
      [
        { opacity: getComputedStyle(overlay).opacity },
        { opacity: entering ? 1 : 0 },
      ],
      {
        duration: duration * (entering ? 180 / 420 : 160 / 360),
        delay: entering ? lead : duration * (1 - 160 / 360),
        easing: "linear",
      },
    );
    return {
      retarget: () => {
        if (cancelled || !entering || geometry.playState === "finished") return;
        const next = windowNode.getBoundingClientRect();
        if (!next.width || !next.height) return;
        const effect = geometry.effect as KeyframeEffect | null;
        if (!effect) return;
        const timing = effect.getTiming();
        const elapsed = Number(geometry.currentTime ?? 0);
        const remaining = Math.max(
          0,
          Number(timing.duration) + (timing.delay ?? 0) - elapsed,
        );
        if (!remaining) return;
        const delay = Math.max(0, (timing.delay ?? 0) - elapsed);
        const computed = getComputedStyle(material);
        // Intrinsic content has already changed the centered root's position.
        // Rebase the *same* decorative track in viewport coordinates before paint.
        const from = rectFrame(
          parseFloat(computed.left) + destination.left - next.left,
          parseFloat(computed.top) + destination.top - next.top,
          parseFloat(computed.width),
          parseFloat(computed.height),
          computed,
        );
        effect.setKeyframes([
          from,
          rectFrame(
            -1,
            -1,
            next.width,
            next.height,
            getComputedStyle(windowNode),
          ),
        ]);
        geometry.currentTime = 0;
        effect.updateTiming({ duration: remaining - delay, delay });
        destination = next;
      },
      finished: Promise.all(animations.map(done)).then(
        (results) => !cancelled && results.every(Boolean),
      ),
      contentFinished: done(content),
      cancel,
    };
  } catch (error) {
    // Attach rejection handlers before cancel() rejects already-started tracks.
    animations.forEach((animation) => {
      void done(animation);
    });
    cancel(false);
    throw error;
  }
}

export function settleDialogPlanes(planes: DialogPlanes): void {
  for (const property of geometryProperties)
    planes.material.style[property] = "";
  planes.material.style.opacity = "1";
  planes.sourceMaterial.style.opacity = "0";
  planes.targetMaterial.style.opacity = "1";
  planes.foreground.style.opacity = "1";
  planes.overlay.style.opacity = "1";
}

/** Same-session layout, not another source entrance. Native interpolation
 * changes only the fixed dialog's size; content/footer are not scale-distorted. */
export function runDialogResize({
  windowNode,
  contentNode,
  from,
  target,
  duration,
}: {
  windowNode: HTMLElement;
  contentNode: HTMLElement;
  from: Pick<DOMRect, "width" | "height">;
  target: Pick<DOMRect, "width" | "height">;
  duration: number;
}) {
  const animations: Animation[] = [];
  let cancelled = false;
  const done = (animation: Animation) =>
    animation.finished.then(
      () => true,
      () => false,
    );
  const cancel = (freeze = false) => {
    if (cancelled) return;
    cancelled = true;
    const box = freeze ? windowNode.getBoundingClientRect() : null;
    animations.forEach((animation) => animation.cancel());
    if (box) {
      windowNode.style.width = `${box.width}px`;
      windowNode.style.height = `${box.height}px`;
    }
  };
  try {
    animations.push(
      windowNode.animate(
        [
          { width: `${from.width}px`, height: `${from.height}px` },
          { width: `${target.width}px`, height: `${target.height}px` },
        ],
        { duration, easing: fySpatialEasing, fill: "both" },
      ),
    );
    // The action row stays visible, so its real press feedback is not covered
    // by a full-dialog fade. No outgoing form/tree is retained for crossfade.
    animations.push(
      contentNode.animate([{ opacity: 0.5 }, { opacity: 1 }], {
        duration,
        easing: fySpatialEasing,
        fill: "both",
      }),
    );
    return {
      cancel,
      finished: Promise.all(animations.map(done)).then(
        (results) => !cancelled && results.every(Boolean),
      ),
    };
  } catch (error) {
    animations.forEach((animation) => {
      void done(animation);
    });
    cancel();
    throw error;
  }
}
