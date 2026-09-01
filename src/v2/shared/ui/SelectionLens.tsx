import {
  createContext,
  useCallback,
  useContext,
  useLayoutEffect,
  useRef,
  useState,
  type HTMLAttributes,
} from "react";

import { classNames } from "../design-system/classNames";
import {
  animate,
  fySpringTransition,
  motion,
  useMotionValue,
  useReducedMotion,
} from "./motion";

import "./selection-lens.css";

export { fySpringTransition as selectionLensTransition };

type LensBox = {
  x: number;
  y: number;
  width: number;
  height: number;
  borderRadius: string;
};

export type SelectionLensGeometry = "size-and-position" | "position";

const layoutSettleFrameCount = 48;

export function selectionLensCollapsedOrigin(box: Pick<LensBox, "x" | "y">): {
  x: number;
  y: number;
  width: number;
  height: number;
} {
  return {
    x: box.x,
    y: box.y,
    width: 0,
    height: 0,
  };
}

type SelectionLensContextValue = {
  register: (host: HTMLElement | null) => void;
  unregister: (host: HTMLElement) => void;
};

const SelectionLensContext = createContext<SelectionLensContextValue | null>(
  null,
);

function isHiddenFromLayout(element: HTMLElement): boolean {
  let node: HTMLElement | null = element;
  while (node) {
    if (node.hidden) return true;
    node = node.parentElement;
  }
  return false;
}

function observeHiddenAncestors(
  start: HTMLElement,
  onChange: () => void,
): () => void {
  if (typeof MutationObserver === "undefined") {
    return () => undefined;
  }
  const observer = new MutationObserver(onChange);
  let node: HTMLElement | null = start;
  while (node) {
    observer.observe(node, {
      attributes: true,
      attributeFilter: ["hidden"],
    });
    node = node.parentElement;
  }
  return () => observer.disconnect();
}

export function SelectionLensGroup({
  id,
  inset = 0,
  geometry = "size-and-position",
  layoutKey,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id"> & {
  id: string;
  inset?: number;
  geometry?: SelectionLensGeometry;
  layoutKey?: string | number | boolean;
}) {
  const scopeRef = useRef<HTMLDivElement>(null);
  const hostRef = useRef<HTMLElement | null>(null);
  const hostGenerationRef = useRef(0);
  const frameRef = useRef<number | null>(null);
  const layoutSettleFrameRef = useRef<number | null>(null);
  const previousLayoutKeyRef = useRef(layoutKey);
  const hiddenRef = useRef(false);
  const positionedRef = useRef(false);
  const revealSeenRef = useRef(0);
  const [host, setHost] = useState<HTMLElement | null>(null);
  const [box, setBox] = useState<LensBox | null>(null);
  const [revealKey, setRevealKey] = useState(0);
  const reduceMotion = useReducedMotion() === true;
  const left = useMotionValue(0);
  const top = useMotionValue(0);
  const width = useMotionValue(0);
  const height = useMotionValue(0);

  const syncBox = useCallback(() => {
    const scope = scopeRef.current;
    const nextHost = hostRef.current;
    if (!scope || !nextHost) {
      return;
    }

    if (isHiddenFromLayout(scope)) {
      hiddenRef.current = true;
      return;
    }

    const scopeRect = scope.getBoundingClientRect();
    const hostRect = nextHost.getBoundingClientRect();
    if (hiddenRef.current) {
      hiddenRef.current = false;
      setRevealKey((key) => key + 1);
    }
    const nextBox = {
      x: hostRect.left - scopeRect.left + inset,
      y: hostRect.top - scopeRect.top + inset,
      width: Math.max(0, hostRect.width - inset * 2),
      height: Math.max(0, hostRect.height - inset * 2),
      borderRadius: getComputedStyle(nextHost).borderRadius,
    };
    setBox((current) =>
      current &&
      current.x === nextBox.x &&
      current.y === nextBox.y &&
      current.width === nextBox.width &&
      current.height === nextBox.height &&
      current.borderRadius === nextBox.borderRadius
        ? current
        : nextBox,
    );
  }, [inset]);

  const scheduleSync = useCallback(() => {
    if (frameRef.current !== null) {
      return;
    }
    frameRef.current = window.requestAnimationFrame(() => {
      frameRef.current = null;
      syncBox();
    });
  }, [syncBox]);

  const register = useCallback((nextHost: HTMLElement | null) => {
    hostGenerationRef.current += 1;
    hostRef.current = nextHost;
    setHost(nextHost);
  }, []);

  const unregister = useCallback((currentHost: HTMLElement) => {
    if (hostRef.current !== currentHost) {
      return;
    }
    const generation = hostGenerationRef.current;
    queueMicrotask(() => {
      if (hostGenerationRef.current !== generation) {
        return;
      }
      if (hostRef.current !== currentHost) {
        return;
      }
      hostRef.current = null;
      positionedRef.current = false;
      setHost(null);
    });
  }, []);

  useLayoutEffect(() => {
    syncBox();
  });

  useLayoutEffect(() => {
    const layoutChanged = previousLayoutKeyRef.current !== layoutKey;
    previousLayoutKeyRef.current = layoutKey;
    if (!layoutChanged) {
      scheduleSync();
      return;
    }

    let remainingFrames = reduceMotion ? 1 : layoutSettleFrameCount;
    const settle = () => {
      layoutSettleFrameRef.current = null;
      syncBox();
      remainingFrames -= 1;
      if (remainingFrames > 0) {
        layoutSettleFrameRef.current = window.requestAnimationFrame(settle);
      }
    };
    layoutSettleFrameRef.current = window.requestAnimationFrame(settle);

    return () => {
      if (layoutSettleFrameRef.current !== null) {
        window.cancelAnimationFrame(layoutSettleFrameRef.current);
        layoutSettleFrameRef.current = null;
      }
    };
  }, [layoutKey, reduceMotion, scheduleSync, syncBox]);

  useLayoutEffect(() => {
    const scope = scopeRef.current;
    if (!scope || !host) {
      return;
    }

    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => {
            scheduleSync();
          });
    observer?.observe(scope);
    observer?.observe(host);
    window.addEventListener("resize", scheduleSync);
    scope.addEventListener("scroll", scheduleSync, true);
    const stopHiddenWatch = observeHiddenAncestors(scope, scheduleSync);
    syncBox();

    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", scheduleSync);
      scope.removeEventListener("scroll", scheduleSync, true);
      stopHiddenWatch();
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
        frameRef.current = null;
      }
    };
  }, [host, scheduleSync, syncBox]);

  useLayoutEffect(() => {
    if (!box) {
      return;
    }

    const collapseToOrigin = () => {
      const origin = selectionLensCollapsedOrigin(box);
      left.set(origin.x);
      top.set(origin.y);
      width.set(geometry === "position" ? box.width : origin.width);
      height.set(geometry === "position" ? box.height : origin.height);
    };

    if (reduceMotion === true) {
      left.set(box.x);
      top.set(box.y);
      width.set(box.width);
      height.set(box.height);
      positionedRef.current = true;
      revealSeenRef.current = revealKey;
      return;
    }

    if (revealKey !== revealSeenRef.current) {
      revealSeenRef.current = revealKey;
      collapseToOrigin();
      positionedRef.current = true;
    } else if (!positionedRef.current) {
      collapseToOrigin();
      positionedRef.current = true;
    }

    if (geometry === "position") {
      width.set(box.width);
      height.set(box.height);
    }

    const controls = [
      animate(left, box.x, fySpringTransition),
      animate(top, box.y, fySpringTransition),
    ];
    if (geometry === "size-and-position") {
      controls.push(
        animate(width, box.width, fySpringTransition),
        animate(height, box.height, fySpringTransition),
      );
    }
    return () => {
      for (const control of controls) {
        control.stop();
      }
    };
  }, [box, geometry, height, left, reduceMotion, revealKey, top, width]);

  return (
    <SelectionLensContext.Provider value={{ register, unregister }}>
      <div
        ref={scopeRef}
        className={classNames("fy-selection-lens-scope", className)}
        data-selection-lens-group={id}
        {...props}
      >
        {children}
        {host && box ? (
          <motion.div
            className="fy-selection-lens"
            style={{
              left,
              top,
              width,
              height,
              borderRadius: box.borderRadius,
            }}
            aria-hidden
            data-testid="selection-lens"
            data-selection-lens-geometry={geometry}
            data-selection-material="frame"
            data-selection-lens-reveal={revealKey}
          />
        ) : null}
      </div>
    </SelectionLensContext.Provider>
  );
}

export function SelectionLens({ active }: { active: boolean }) {
  const ctx = useContext(SelectionLensContext);
  const markerRef = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    if (!ctx || !active) {
      return;
    }

    const host = markerRef.current?.parentElement ?? null;
    ctx.register(host);
    return () => {
      if (host) {
        ctx.unregister(host);
      }
    };
  }, [active, ctx]);

  if (!ctx || !active) {
    return null;
  }

  return (
    <span
      ref={markerRef}
      className="fy-selection-lens-target"
      aria-hidden
      data-selection-lens-target=""
    />
  );
}

export function SelectionLensTrack({
  id,
  geometry,
  layoutKey,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id"> & {
  id: string;
  geometry?: SelectionLensGeometry;
  layoutKey?: string | number | boolean;
}) {
  return (
    <SelectionLensGroup
      id={id}
      geometry={geometry}
      layoutKey={layoutKey}
      className={className}
      {...props}
    >
      {children}
    </SelectionLensGroup>
  );
}
