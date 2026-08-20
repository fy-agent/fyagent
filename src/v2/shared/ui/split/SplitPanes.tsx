import {
  Children,
  Fragment,
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from "react";

import { classNames } from "../../design-system/classNames";

import "./split.css";

export const SPLIT_GAP = 14;
export const SPLIT_RESIZE_STEP = 16;
export const SPLIT_STACK_QUERY = "(max-width: 760px)";
export const SPLIT_COMPACT_THREE_QUERY = "(max-width: 1180px)";

const MIN_WIDTHS_ONE = [0];
const MIN_WIDTHS_TWO = [220, 360];
const MIN_WIDTHS_THREE = [220, 330, 220];
const DEFAULT_LEADING_MAX = 420;
const EMPTY_MAX_WIDTHS: Array<number | undefined> = [];
const EMPTY_PANE_VARS: string[] = [];

type SplitPanesStyle = CSSProperties & Record<string, string | undefined>;

interface SplitPanesProps {
  children: ReactNode;
  className?: string;
  minWidths?: number[];
  maxWidths?: Array<number | undefined>;
  separatorLabels?: string[];
  paneCssVars?: string[];
}

function isSplitStacked(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia(SPLIT_STACK_QUERY).matches
  );
}

function isCompactThree(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia(SPLIT_COMPACT_THREE_QUERY).matches
  );
}

function defaultMinWidths(paneCount: number): number[] {
  if (paneCount <= 1) return MIN_WIDTHS_ONE;
  if (paneCount === 2) return MIN_WIDTHS_TWO;
  if (paneCount === 3) return MIN_WIDTHS_THREE;
  const mins = [...MIN_WIDTHS_THREE];
  while (mins.length < paneCount) {
    mins.push(220);
  }
  return mins;
}

function defaultSeparatorLabels(paneCount: number): string[] {
  if (paneCount <= 2) return ["调整两栏宽度"];
  return ["调整列表与详情的宽度", "调整详情与侧栏的宽度"];
}

function clampLeadingWidth({
  index,
  proposed,
  leading,
  mins,
  maxes,
  containerWidth,
  paneCount,
}: {
  index: number;
  proposed: number;
  leading: number[];
  mins: number[];
  maxes: Array<number | undefined>;
  containerWidth: number;
  paneCount: number;
}): number {
  const minW = mins[index] ?? 0;
  if (!Number.isFinite(proposed)) return minW;
  const explicitMax = maxes[index];
  const gaps = SPLIT_GAP * Math.max(0, paneCount - 1);
  const otherLeading = leading.reduce(
    (sum, width, current) => (current === index ? sum : sum + width),
    0,
  );
  const lastMin = mins[paneCount - 1] ?? 0;
  const maxByContainer =
    containerWidth > 0
      ? containerWidth - gaps - otherLeading - lastMin
      : (explicitMax ?? DEFAULT_LEADING_MAX);
  const maxW = Math.max(
    minW,
    Math.min(explicitMax ?? Number.POSITIVE_INFINITY, maxByContainer),
  );
  return Math.min(maxW, Math.max(minW, Math.round(proposed)));
}

function measureLeadingWidths(
  container: HTMLElement,
  paneCount: number,
  mins: number[],
): number[] {
  const panes = container.querySelectorAll<HTMLElement>(
    ":scope > .fy-split-pane",
  );
  const leadingCount = Math.max(0, paneCount - 1);
  const widths: number[] = [];
  for (let index = 0; index < leadingCount; index += 1) {
    const width = panes[index]?.getBoundingClientRect().width ?? 0;
    widths.push(width > 0 ? Math.round(width) : (mins[index] ?? 0));
  }
  return widths;
}

function buildStyle(
  widths: Array<number | null>,
  paneCssVars: string[],
): SplitPanesStyle | undefined {
  const style: SplitPanesStyle = {};
  let assigned = false;
  widths.forEach((width, index) => {
    if (width === null) return;
    assigned = true;
    style[`--fy-split-pane-${index}`] = `${width}px`;
    const alias = paneCssVars[index];
    if (alias) style[alias] = `${width}px`;
  });
  return assigned ? style : undefined;
}

export function SplitPanes({
  children,
  className,
  minWidths: minWidthsProp,
  maxWidths: maxWidthsProp = EMPTY_MAX_WIDTHS,
  separatorLabels,
  paneCssVars = EMPTY_PANE_VARS,
}: SplitPanesProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{
    index: number;
    startX: number;
    startWidth: number;
    leading: number[];
  } | null>(null);
  const [leadingWidths, setLeadingWidths] = useState<Array<number | null>>([]);
  const [resizing, setResizing] = useState(false);
  const panes = Children.toArray(children);
  const paneCount = panes.length;
  const minWidths = minWidthsProp ?? defaultMinWidths(paneCount);
  const labels = separatorLabels ?? defaultSeparatorLabels(paneCount);
  const leadingCount = Math.max(0, paneCount - 1);

  const applyDrag = useCallback(
    (clientX: number) => {
      const drag = dragRef.current;
      const container = containerRef.current;
      if (!drag || !container) return;
      const nextWidth = clampLeadingWidth({
        index: drag.index,
        proposed: drag.startWidth + (clientX - drag.startX),
        leading: drag.leading.map((width, index) =>
          index === drag.index ? drag.startWidth : width,
        ),
        mins: minWidths,
        maxes: maxWidthsProp,
        containerWidth: container.getBoundingClientRect().width,
        paneCount,
      });
      setLeadingWidths((current) => {
        const next =
          current.length === leadingCount
            ? current.slice()
            : Array.from({ length: leadingCount }, () => null);
        next[drag.index] = nextWidth;
        return next;
      });
    },
    [leadingCount, maxWidthsProp, minWidths, paneCount],
  );

  const endResize = useCallback(() => {
    if (!dragRef.current && !resizing) return;
    dragRef.current = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    setResizing(false);
  }, [resizing]);

  useEffect(() => {
    if (!resizing) return;
    const onMove = (event: PointerEvent) => applyDrag(event.clientX);
    const onUp = () => endResize();
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("pointercancel", onUp);
    return () => {
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
      document.removeEventListener("pointercancel", onUp);
    };
  }, [applyDrag, endResize, resizing]);

  const handleInactive = useCallback(
    (index: number) =>
      isSplitStacked() || (paneCount >= 3 && index >= 1 && isCompactThree()),
    [paneCount],
  );

  const beginResize = useCallback(
    (index: number, event: ReactPointerEvent<HTMLDivElement>) => {
      if (event.button === 1 || event.button === 2 || handleInactive(index)) {
        return;
      }
      const container = containerRef.current;
      if (!container) return;
      const measured = measureLeadingWidths(container, paneCount, minWidths);
      dragRef.current = {
        index,
        startX: event.clientX,
        startWidth: leadingWidths[index] ?? measured[index] ?? minWidths[index],
        leading: measured,
      };
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
      setResizing(true);
    },
    [handleInactive, leadingWidths, minWidths, paneCount],
  );

  const onHandlePointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      applyDrag(event.clientX);
    },
    [applyDrag],
  );

  const onResizeKeyDown = useCallback(
    (index: number, event: ReactKeyboardEvent<HTMLDivElement>) => {
      if (handleInactive(index)) return;
      const container = containerRef.current;
      if (!container) return;
      const measured = measureLeadingWidths(container, paneCount, minWidths);
      const current =
        leadingWidths[index] ?? measured[index] ?? minWidths[index];
      const leading = measured.map((width, currentIndex) =>
        currentIndex === index ? current : width,
      );
      const containerWidth = container.getBoundingClientRect().width;
      let proposed: number;
      switch (event.key) {
        case "ArrowLeft":
          proposed = current - SPLIT_RESIZE_STEP;
          break;
        case "ArrowRight":
          proposed = current + SPLIT_RESIZE_STEP;
          break;
        case "Home":
          proposed = minWidths[index] ?? 0;
          break;
        case "End":
          proposed = maxWidthsProp[index] ?? DEFAULT_LEADING_MAX;
          break;
        default:
          return;
      }
      event.preventDefault();
      const nextWidth = clampLeadingWidth({
        index,
        proposed,
        leading,
        mins: minWidths,
        maxes: maxWidthsProp,
        containerWidth,
        paneCount,
      });
      setLeadingWidths((widths) => {
        const next =
          widths.length === leadingCount
            ? widths.slice()
            : Array.from({ length: leadingCount }, () => null);
        next[index] = nextWidth;
        return next;
      });
    },
    [
      handleInactive,
      leadingCount,
      leadingWidths,
      maxWidthsProp,
      minWidths,
      paneCount,
    ],
  );

  const resetPane = useCallback((index: number) => {
    setLeadingWidths((widths) => {
      if (widths.length === 0) return widths;
      const next = widths.slice();
      next[index] = null;
      return next;
    });
  }, []);

  return (
    <div
      ref={containerRef}
      className={classNames("fy-split-panes", className)}
      data-panes={paneCount}
      data-resizing={resizing ? "true" : undefined}
      style={buildStyle(leadingWidths, paneCssVars)}
    >
      {panes.map((pane, index) => (
        <Fragment key={index}>
          {index > 0 ? (
            <SplitResizeHandle
              index={index - 1}
              label={labels[index - 1] ?? `调整第 ${index} 栏宽度`}
              max={maxWidthsProp[index - 1] ?? DEFAULT_LEADING_MAX}
              min={minWidths[index - 1] ?? 0}
              valueNow={leadingWidths[index - 1] ?? minWidths[index - 1] ?? 0}
              onPointerDown={(event) => beginResize(index - 1, event)}
              onPointerMove={onHandlePointerMove}
              onPointerUp={endResize}
              onKeyDown={(event) => onResizeKeyDown(index - 1, event)}
              onReset={() => resetPane(index - 1)}
            />
          ) : null}
          <div className="fy-split-pane" data-index={index}>
            {pane}
          </div>
        </Fragment>
      ))}
    </div>
  );
}

function SplitResizeHandle({
  index,
  label,
  max,
  min,
  valueNow,
  onPointerDown,
  onPointerMove,
  onPointerUp,
  onKeyDown,
  onReset,
}: {
  index: number;
  label: string;
  max: number;
  min: number;
  valueNow: number;
  onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerMove: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerUp: () => void;
  onKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void;
  onReset: () => void;
}) {
  return (
    <div
      className="fy-split-resize-handle"
      data-index={index}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={valueNow}
      aria-valuetext={`${valueNow} 像素`}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onKeyDown={onKeyDown}
      onDoubleClick={onReset}
    />
  );
}
