import {
  createContext,
  useCallback,
  useContext,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type HTMLAttributes,
} from "react";

import { classNames } from "../design-system/classNames";
import {
  animate,
  fySelectionTransition,
  motion,
  useMotionValue,
  useReducedMotion,
} from "./motion";

import "./selection-lens.css";

export { fySelectionTransition as selectionLensTransition };

type LensBox = {
  x: number;
  y: number;
  width: number;
  height: number;
  borderRadius: string;
  layoutChange?: boolean;
};

export type SelectionLensGeometry = "size-and-position" | "position";

type SelectionLensContextValue = {
  register: (host: HTMLElement | null) => void;
  unregister: (host: HTMLElement) => void;
};

const SelectionLensContext = createContext<SelectionLensContextValue | null>(
  null,
);

function roundBox(box: LensBox): LensBox {
  const dpr =
    typeof window === "undefined" || window.devicePixelRatio === 0
      ? 1
      : window.devicePixelRatio;
  const snap = (value: number) => Math.round(value * dpr) / dpr;
  return {
    x: snap(box.x),
    y: snap(box.y),
    width: snap(box.width),
    height: snap(box.height),
    borderRadius: box.borderRadius,
    layoutChange: box.layoutChange,
  };
}

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
  const measuredHostRef = useRef<HTMLElement | null>(null);
  const hostGenerationRef = useRef(0);
  const frameRef = useRef<number | null>(null);
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

  const syncBox = useCallback(
    (layoutChange = false) => {
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
      const selectionChanged = measuredHostRef.current !== nextHost;
      measuredHostRef.current = nextHost;
      const wasHidden = hiddenRef.current;
      if (wasHidden) {
        hiddenRef.current = false;
        setRevealKey((key) => key + 1);
      }
      const nextBox = roundBox({
        x: hostRect.left - scopeRect.left + inset,
        y: hostRect.top - scopeRect.top + inset,
        width: Math.max(0, hostRect.width - inset * 2),
        height: Math.max(0, hostRect.height - inset * 2),
        borderRadius: getComputedStyle(nextHost).borderRadius,
        // An observer scheduled for the old selection can fire after a new host
        // registers. Its first measurement is still selection travel, not reflow.
        layoutChange: (layoutChange && !selectionChanged) || wasHidden,
      });
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
    },
    [inset],
  );

  const scheduleSync = useCallback(() => {
    const scope = scopeRef.current;
    if (scope && isHiddenFromLayout(scope)) {
      hiddenRef.current = true;
      return;
    }
    if (frameRef.current !== null) {
      return;
    }
    frameRef.current = window.requestAnimationFrame(() => {
      frameRef.current = null;
      syncBox(true);
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
    const scope = scopeRef.current;
    if (!scope) {
      return;
    }
    if (isHiddenFromLayout(scope)) {
      hiddenRef.current = true;
      return;
    }
    if (hiddenRef.current) {
      scheduleSync();
    }
  });

  useLayoutEffect(() => {
    scheduleSync();
  }, [layoutKey, scheduleSync]);

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
    // A sibling section can move a selected link without resizing the outer
    // track or the link. Observe actual layout blocks, not a fixed RAF budget.
    Array.from(scope.children).forEach((child) => {
      if (!child.classList.contains("fy-selection-lens"))
        observer?.observe(child);
    });
    const collapseObserver =
      !observer && typeof MutationObserver !== "undefined"
        ? new MutationObserver(scheduleSync)
        : null;
    scope.querySelectorAll(".fy-collapsible-panel").forEach((panel) => {
      collapseObserver?.observe(panel, {
        attributes: true,
        attributeFilter: ["style"],
      });
    });
    window.addEventListener("resize", scheduleSync);
    scope.addEventListener("scroll", scheduleSync, true);
    const stopHiddenWatch = observeHiddenAncestors(scope, scheduleSync);
    syncBox();

    return () => {
      observer?.disconnect();
      collapseObserver?.disconnect();
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

    // Only an actual selection moves between hosts. Initial/revisited geometry
    // and layout corrections must not replay a zero-size grow or restart a
    // tween on every frame of a neighbouring collapse.
    if (
      reduceMotion ||
      !positionedRef.current ||
      box.layoutChange ||
      revealKey !== revealSeenRef.current
    ) {
      left.set(box.x);
      top.set(box.y);
      width.set(box.width);
      height.set(box.height);
      positionedRef.current = true;
      revealSeenRef.current = revealKey;
      return;
    }

    if (geometry === "position") {
      width.set(box.width);
      height.set(box.height);
    }

    const controls = [
      animate(left, box.x, fySelectionTransition),
      animate(top, box.y, fySelectionTransition),
    ];
    if (geometry === "size-and-position") {
      controls.push(
        animate(width, box.width, fySelectionTransition),
        animate(height, box.height, fySelectionTransition),
      );
    }
    return () => {
      for (const control of controls) {
        control.stop();
      }
    };
  }, [box, geometry, height, left, reduceMotion, revealKey, top, width]);

  const context = useMemo(
    () => ({ register, unregister }),
    [register, unregister],
  );
  return (
    <SelectionLensContext.Provider value={context}>
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
              left: 0,
              top: 0,
              x: left,
              y: top,
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
