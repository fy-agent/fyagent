import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { classNames } from "../design-system/classNames";
import { Button } from "./Button";
import { FrostedSurface } from "./GlassMaterial";
import { dialogOriginGeometry, type DialogOriginRef } from "./dialogOrigin";
import {
  animate,
  AnimatePresence,
  fySurfaceEase,
  motionDuration,
  usePresence,
  useReducedMotion,
} from "./motion";
import { usePersistentVisibility } from "./PersistentSurface";
import { DialogPrimitive } from "./vendor";

export interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children?: ReactNode;
  actions?: ReactNode;
  size?: "standard" | "comfortable" | "wide";
  initialFocusRef?: RefObject<HTMLElement>;
  originRef?: DialogOriginRef;
}

export function Dialog(props: DialogProps) {
  const visible = usePersistentVisibility();
  // A hidden route is not a user close: remove its portal immediately.
  if (!visible) return null;
  return (
    <AnimatePresence propagate>
      {props.open && <DialogLayer key="dialog" {...props} />}
    </AnimatePresence>
  );
}

function DialogLayer({
  onOpenChange,
  title,
  description,
  children,
  actions,
  size = "standard",
  initialFocusRef,
  originRef,
}: DialogProps) {
  const [present, safeToRemove] = usePresence();
  const reduce = useReducedMotion();
  // Radix mounts its portal after the parent layer's initial layout effect.
  // Track the committed node, rather than assuming the ref exists on mount.
  const contentRef = useRef<HTMLDivElement | null>(null);
  const committedNodeRef = useRef<HTMLDivElement | null>(null);
  const [mountVersion, setMountVersion] = useState(0);
  const setContent = useCallback((node: HTMLDivElement | null) => {
    if (contentRef.current === node) return;
    contentRef.current = node;
    // Radix can detach/recompose refs without replacing the underlying node.
    // Those transient null callbacks must not create another mount generation.
    if (node && committedNodeRef.current !== node) {
      committedNodeRef.current = node;
      setMountVersion((version) => version + 1);
    }
  }, []);
  const backing = useRef<HTMLDivElement>(null);
  const foreground = useRef<HTMLDivElement>(null);
  const overlay = useRef<HTMLDivElement>(null);
  const lastBox = useRef<DOMRect | null>(null);
  const source = useRef<HTMLElement | null>(null);
  const wasPresent = useRef(false);
  const hasAnimated = useRef(false);
  const epoch = useRef(0);
  const resizeTransition = useRef<(() => void) | null>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const restoreFrameRef = useRef<number | null>(null);
  const phaseKey = `${mountVersion}:${present}:${reduce}`;
  const [settledPhase, setSettledPhase] = useState<string | null>(null);
  const enhanced = present && settledPhase === phaseKey;
  const remove = useRef(safeToRemove);
  const requestedSource = useRef(originRef);
  useLayoutEffect(() => {
    remove.current = safeToRemove;
    requestedSource.current = originRef;
  }, [safeToRemove, originRef]);

  useLayoutEffect(() => {
    const element = contentRef.current;
    if (!element || !present) return;
    const measure = () => {
      const next = element.getBoundingClientRect();
      const previous = lastBox.current;
      lastBox.current = next;
      if (
        previous &&
        (previous.width !== next.width || previous.height !== next.height)
      )
        resizeTransition.current?.();
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [present, mountVersion]);

  useLayoutEffect(() => {
    const element = contentRef.current,
      material = backing.current,
      text = foreground.current,
      scrim = overlay.current;
    if (!element || !material || !text || !scrim) return;
    const revision = ++epoch.current;
    if (present && !wasPresent.current)
      source.current = requestedSource.current?.current ?? null;
    wasPresent.current = present;
    if (!present && lastBox.current?.height) {
      // Discard form DOM immediately; only its inert material retains geometry.
      element.style.height = `${lastBox.current.height}px`;
      element.style.width = `${Math.min(lastBox.current.width, innerWidth - 32)}px`;
    } else {
      element.style.removeProperty("height");
      element.style.removeProperty("width");
    }
    const box = element.getBoundingClientRect();
    const origin = dialogOriginGeometry(source.current, box);
    element.dataset.motionOrigin = origin.sourced ? "trigger" : "neutral";
    const duration = motionDuration(present ? "dialog-enter" : "dialog-exit");
    const settle = () => {
      material.style.transform = "none";
      material.style.opacity = "1";
      text.style.transform = "none";
      text.style.opacity = "1";
      scrim.style.opacity = "1";
      // Presence records exiting keys in its parent layout effect. Completion
      // crosses that commit barrier and cannot complete a superseded phase.
      void Promise.resolve().then(() => {
        if (revision !== epoch.current) return;
        if (!present) remove.current?.();
        else if (box.width && box.height) setSettledPhase(phaseKey);
      });
    };
    // A zero-layout surface cannot produce meaningful spatial animation.
    if (reduce || !box.width || !box.height || duration === 0) {
      settle();
      return () => {
        epoch.current += 1;
      };
    }
    const first = !hasAnimated.current;
    hasAnimated.current = true;
    const ease: [number, number, number, number] = [...fySurfaceEase];
    const animations: ReturnType<typeof animate>[] = [];
    try {
      if (present) {
        animations.push(
          animate(
            material,
            {
              x: [first ? origin.x : null, 0],
              y: [first ? origin.y : null, 0],
              scaleX: [first ? origin.scaleX : null, 1],
              scaleY: [first ? origin.scaleY : null, 1],
              opacity: [first ? 0 : null, 1],
            },
            { duration, ease },
          ),
        );
        animations.push(
          animate(
            text,
            { opacity: [first ? 0 : null, 1], y: [first ? 6 : null, 0] },
            {
              duration: motionDuration("content"),
              delay: first ? duration * 0.22 : 0,
              ease,
            },
          ),
        );
        animations.push(
          animate(
            scrim,
            { opacity: [first ? 0 : null, 1] },
            { duration: motionDuration("content"), ease },
          ),
        );
      } else {
        animations.push(
          animate(
            material,
            {
              x: origin.x,
              y: origin.y,
              scaleX: origin.scaleX,
              scaleY: origin.scaleY,
              opacity: 0,
            },
            { duration, ease },
          ),
        );
        animations.push(
          animate(text, { opacity: 0 }, { duration: duration * 0.35, ease }),
        );
        animations.push(animate(scrim, { opacity: 0 }, { duration, ease }));
      }
    } catch {
      // A later layer can fail after an earlier animation has started. Stop
      // every started handle before settling; never leave an orphan writer.
      animations.forEach((animation) => animation.stop());
      console.warn("Dialog animation unavailable; settled without motion");
      settle();
      return;
    }
    void Promise.all(animations).then(
      () => {
        if (revision !== epoch.current) return;
        if (present) setSettledPhase(phaseKey);
        else {
          // Presence registers descendants in passive effects. Complete a zero-
          // duration exit after registration, rather than racing it in layout.
          const completionEpoch = epoch.current;
          void Promise.resolve().then(() => {
            if (epoch.current === completionEpoch) remove.current?.();
          });
        }
      },
      () => {
        if (revision === epoch.current) settle();
      },
    );
    const resized = () => {
      animations.forEach((animation) => animation.stop());
      settle();
    };
    resizeTransition.current = resized;
    window.addEventListener("resize", resized);
    return () => {
      epoch.current += 1;
      animations.forEach((animation) => animation.stop());
      resizeTransition.current = null;
      window.removeEventListener("resize", resized);
    };
  }, [present, reduce, mountVersion, phaseKey]);

  return (
    // Radix retains modal/scroll/focus ownership until Motion removes the layer.
    // Business open state is already false while its decorative material exits.
    <DialogPrimitive.Root
      open
      onOpenChange={(next) => {
        if (present) onOpenChange(next);
      }}
    >
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          ref={overlay}
          className="fy-control-dialog-overlay"
        />
        <DialogPrimitive.Content
          ref={setContent}
          data-motion-phase={present ? "open" : "exit"}
          data-motion-settled={present && enhanced ? "true" : undefined}
          className={classNames(
            "fy-control-dialog",
            size !== "standard" && `fy-control-dialog-${size}`,
          )}
          {...(!description ? { "aria-describedby": undefined } : {})}
          onPointerDownCapture={(event) => {
            if (!present) {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
          onClickCapture={(event) => {
            if (!present) {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
          onKeyDownCapture={(event) => {
            if (!present && event.key !== "Tab") {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
          onOpenAutoFocus={(event) => {
            if (restoreFrameRef.current !== null)
              window.cancelAnimationFrame(restoreFrameRef.current);
            const focused = originRef?.current ?? document.activeElement;
            if (focused instanceof HTMLElement && focused !== document.body)
              restoreFocusRef.current = focused;
            const initial = initialFocusRef?.current;
            if (initial && !initial.matches(":disabled")) {
              event.preventDefault();
              initial.focus();
            }
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            const origin = restoreFocusRef.current;
            if (restoreFrameRef.current !== null)
              window.cancelAnimationFrame(restoreFrameRef.current);
            restoreFrameRef.current = window.requestAnimationFrame(() => {
              restoreFrameRef.current = null;
              const node = origin?.matches(
                '[role="tab"][aria-selected="false"]',
              )
                ? origin
                    .closest('[role="tablist"]')
                    ?.querySelector<HTMLElement>(
                      '[role="tab"][aria-selected="true"]',
                    )
                : origin;
              if (
                !node?.isConnected ||
                node.closest("[hidden], [inert]") ||
                node.matches(":disabled")
              )
                return;
              const dialogs = document.querySelectorAll(
                '[role="dialog"][data-state="open"]',
              );
              if (Array.from(dialogs).some((dialog) => !dialog.contains(node)))
                return;
              node.focus({ preventScroll: true });
            });
          }}
        >
          <div ref={backing} className="fy-dialog-material" aria-hidden>
            <FrostedSurface enhanced={enhanced} />
          </div>
          <div ref={foreground} className="fy-dialog-foreground">
            <div className="fy-control-dialog-content">
              <header className="fy-control-dialog-header">
                <DialogPrimitive.Title className="fy-control-dialog-title">
                  {title}
                </DialogPrimitive.Title>
                {description && (
                  <DialogPrimitive.Description className="fy-control-dialog-description">
                    {description}
                  </DialogPrimitive.Description>
                )}
              </header>
              {present && children != null && (
                <div className="fy-control-dialog-body">{children}</div>
              )}
            </div>
            {present && actions && (
              <footer className="fy-control-dialog-actions">{actions}</footer>
            )}
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

export function ConfirmDialog({
  open,
  title,
  description,
  pending,
  onConfirm,
  onCancel,
  originRef,
}: {
  open: boolean;
  title: string;
  description: string;
  pending?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
  originRef?: DialogOriginRef;
}) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  return (
    <Dialog
      open={open}
      originRef={originRef}
      initialFocusRef={cancelRef}
      onOpenChange={(next) => !next && !pending && onCancel()}
      title={title}
      description={description}
      actions={
        <>
          <Button ref={cancelRef} onClick={onCancel} disabled={pending}>
            取消
          </Button>
          <Button
            className="fy-control-button-danger"
            onClick={onConfirm}
            disabled={pending}
          >
            {pending ? "处理中…" : "确认"}
          </Button>
        </>
      }
    />
  );
}
