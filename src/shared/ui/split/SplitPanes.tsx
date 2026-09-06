import {
  Children,
  Fragment,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { classNames } from "../../design-system/classNames";
import {
  ResizableGroup,
  ResizablePanel,
  ResizeSeparator,
  type PanelImperativeHandle,
} from "./vendor";
import "./split.css";
import type { SplitSizing } from "./sizing";

export const SPLIT_GAP = 14;
const TWO_MINIMUMS = [220, 360];
const THREE_MINIMUMS = [220, 330, 220];
const NO_MAXIMUMS: Array<number | undefined> = [];
const NO_ALIASES: string[] = [];

interface SplitPanesProps extends SplitSizing {
  children: ReactNode;
  className?: string;
  separatorLabels?: string[];
  paneCssVars?: string[];
}

/** Product dimensions and labels wrap the library's pointer/keyboard/ARIA owner.
 * The Panel tree never changes at a breakpoint, so editors keep their drafts. */
export function SplitPanes({
  children,
  className,
  minWidths: requestedMinimums,
  maxWidths = NO_MAXIMUMS,
  defaultWidths = NO_MAXIMUMS,
  flexiblePane,
  separatorLabels,
  paneCssVars = NO_ALIASES,
}: SplitPanesProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const id = useId();
  const panelsRef = useRef<Array<PanelImperativeHandle | null>>([]);
  const panes = Children.toArray(children);
  const flexibleIndex = flexiblePane ?? panes.length - 1;
  const minimums =
    requestedMinimums ?? (panes.length === 3 ? THREE_MINIMUMS : TWO_MINIMUMS);
  const requiredWidth =
    panes.reduce<number>((sum, _, index) => sum + (minimums[index] ?? 220), 0) +
    SPLIT_GAP * Math.max(0, panes.length - 1);
  const [stacked, setStacked] = useState(false);
  const [hasLayout, setHasLayout] = useState(false);
  const canResize = typeof ResizeObserver !== "undefined";
  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const measure = () => {
      const width = root.getBoundingClientRect().width;
      setHasLayout(width > 0);
      // Hidden keep-alive surfaces have no usable size. Keep their last layout.
      if (width > 0) setStacked(width < requiredWidth);
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(root);
    return () => observer.disconnect();
  }, [requiredWidth]);

  const defaultWidth = (index: number) =>
    Math.min(
      maxWidths[index] ?? Infinity,
      defaultWidths[index] ??
        (minimums[index] ?? 220) + (index === 0 ? 48 : 70),
    );
  return (
    <div
      ref={rootRef}
      className={classNames("fy-split-panes", className)}
      data-panes={panes.length}
      data-flexible-pane={flexibleIndex}
      data-stacked={stacked || !canResize ? "true" : "false"}
      data-resize-fallback={!canResize ? "static" : undefined}
    >
      {!canResize ? (
        panes.map((pane, index) => (
          <div key={index} className="fy-split-pane" data-index={index}>
            {pane}
          </div>
        ))
      ) : (
        <ResizableGroup
          className="fy-split-group"
          orientation={stacked ? "vertical" : "horizontal"}
          disabled={stacked || !hasLayout}
          style={stacked ? { minHeight: `${panes.length * 260}px` } : undefined}
          resizeTargetMinimumSize={{ fine: SPLIT_GAP, coarse: 28 }}
        >
          {panes.map((pane, index) => (
            <Fragment key={index}>
              {index > 0 && (
                <ResizeSeparator
                  className="fy-split-resize-handle"
                  aria-label={
                    separatorLabels?.[index - 1] ?? `调整第 ${index} 栏宽度`
                  }
                  data-index={index - 1}
                  disabled={stacked || !hasLayout}
                  disableDoubleClick
                  onDoubleClick={() => {
                    const resetIndex =
                      index - 1 === flexibleIndex ? index : index - 1;
                    if (!stacked && hasLayout)
                      panelsRef.current[resetIndex]?.resize(
                        defaultWidth(resetIndex),
                      );
                  }}
                />
              )}
              <ResizablePanel
                id={`${id}-pane-${index}`}
                className="fy-split-pane"
                data-index={index}
                panelRef={(panel) => {
                  panelsRef.current[index] = panel;
                }}
                minSize={stacked ? 220 : (minimums[index] ?? 220)}
                maxSize={stacked ? undefined : maxWidths[index]}
                defaultSize={
                  stacked
                    ? `${100 / panes.length}%`
                    : index !== flexibleIndex
                      ? defaultWidth(index)
                      : undefined
                }
                groupResizeBehavior={
                  !stacked && index !== flexibleIndex
                    ? "preserve-pixel-size"
                    : "preserve-relative-size"
                }
                onResize={(size) => {
                  // Compatibility styling aliases report library state; they do not
                  // drive another resize loop or store a second layout authority.
                  if (!stacked && paneCssVars[index])
                    rootRef.current?.style.setProperty(
                      paneCssVars[index],
                      `${size.inPixels}px`,
                    );
                }}
              >
                {pane}
              </ResizablePanel>
            </Fragment>
          ))}
        </ResizableGroup>
      )}
    </div>
  );
}
