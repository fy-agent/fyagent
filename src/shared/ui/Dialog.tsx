import {
  useCallback,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
  type MouseEventHandler,
} from "react";
import { classNames } from "../design-system/classNames";
import { Button } from "./Button";
import { FrostedSurface } from "./GlassMaterial";
import {
  dialogOriginGeometry,
  type DialogOriginRef,
  type DialogOriginSnapshot,
} from "./dialogOrigin";
import {
  runDialogPresentation,
  settleDialogPlanes,
  type DialogPlanes,
} from "./dialogPresentation";
import {
  AnimatePresence,
  motionDuration,
  usePresence,
  useReducedMotion,
} from "./motion";
import { usePersistentVisibility } from "./PersistentSurface";
import { DialogPrimitive } from "./vendor";
import { useDialogResize } from "./useDialogResize";

export interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children?: ReactNode;
  actions?: ReactNode;
  size?: "standard" | "comfortable" | "wide";
  initialFocusRef?: RefObject<HTMLElement>;
  originRef: DialogOriginRef | undefined;
  /** Closed presentation stage, not a new editor/session identity. */
  presentationKey?: string | number;
  /** Only reviewed non-credential presentation can opt in. Interaction is
   * revoked immediately; original content may fade for at most 80ms. */
  exitContent?: "clear" | "fade";
}

export function Dialog(props: DialogProps) {
  const visible = usePersistentVisibility();
  const [lifetime, setLifetime] = useState({
    open: props.open,
    visible,
    hasLayer: props.open && visible,
  });
  // Preserve participation through a real exit, but never enroll an empty,
  // closed sibling in its parent's completion barrier. Adjust before children
  // render, rather than cascading an effect or changing the editor's key.
  if (lifetime.open !== props.open || lifetime.visible !== visible) {
    setLifetime({
      open: props.open,
      visible,
      hasLayer: visible && (props.open || lifetime.hasLayer),
    });
  }
  if (!visible) return null;
  return (
    <AnimatePresence
      propagate={props.open || lifetime.hasLayer}
      onExitComplete={() =>
        setLifetime((current) => ({ ...current, hasLayer: false }))
      }
    >
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
  presentationKey,
  exitContent = "clear",
}: DialogProps) {
  const [present, safeToRemove] = usePresence();
  const reduce = useReducedMotion();
  const contentRef = useRef<HTMLDivElement | null>(null);
  const committedNodeRef = useRef<HTMLDivElement | null>(null);
  const [mountVersion, setMountVersion] = useState(0);
  const setContent = useCallback((node: HTMLDivElement | null) => {
    contentRef.current = node;
    // Ignore Radix's transient ref recomposition of the same mounted node.
    if (node && committedNodeRef.current !== node) {
      committedNodeRef.current = node;
      setMountVersion((version) => version + 1);
    }
  }, []);
  const materialRef = useRef<HTMLDivElement>(null);
  const sourceMaterialRef = useRef<HTMLDivElement>(null);
  const targetMaterialRef = useRef<HTMLDivElement>(null);
  const foregroundRef = useRef<HTMLDivElement>(null);
  const bodyPresentationRef = useRef<HTMLDivElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const lastBox = useRef<DOMRect | null>(null);
  const source = useRef<HTMLElement | null>(null);
  const returnSource = useRef<HTMLElement | null>(null);
  const capturedSource = useRef<DialogOriginSnapshot>();
  const wasPresent = useRef(false);
  const hasAnimated = useRef(false);
  const epoch = useRef(0);
  const resizeTransition = useRef<(() => void) | null>(null);
  const originRetargetRef = useRef<(() => void) | null>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const restoreFrameRef = useRef<number | null>(null);
  // A true→false→true cycle must not reuse an already-settled boolean key.
  const phase = useMemo(
    () => ({ present, reduce, mountVersion }),
    [present, reduce, mountVersion],
  );
  const [settledPhase, setSettledPhase] = useState<typeof phase | null>(null);
  const [retiredPhase, setRetiredPhase] = useState<typeof phase | null>(null);
  const settled = settledPhase === phase;
  const nativeAnimation =
    typeof Element !== "undefined" &&
    typeof Element.prototype.animate === "function";
  const interactive = present && (settled || reduce || !nativeAnimation);
  const {
    resizing,
    settle: settleContentResize,
    freeze: freezeContentResize,
  } = useDialogResize({
    rootRef: contentRef,
    bodyRef: bodyPresentationRef,
    lastBoxRef: lastBox,
    originSettler: resizeTransition,
    originRetargetRef,
    present,
    settled,
    reduce,
    nativeAnimation,
    mountVersion,
    presentationKey: `${size}:${presentationKey ?? ""}`,
  });
  const keepBody =
    present || (exitContent === "fade" && retiredPhase !== phase);
  const remove = useRef(safeToRemove);
  const requestedSource = useRef(originRef);
  useLayoutEffect(() => {
    remove.current = safeToRemove;
    requestedSource.current = originRef;
  }, [safeToRemove, originRef]);

  useLayoutEffect(() => {
    const element = contentRef.current;
    if (
      !element ||
      !mountVersion ||
      !materialRef.current ||
      !sourceMaterialRef.current ||
      !targetMaterialRef.current ||
      !foregroundRef.current ||
      !overlayRef.current
    )
      return;
    const planes: DialogPlanes = {
      material: materialRef.current,
      sourceMaterial: sourceMaterialRef.current,
      targetMaterial: targetMaterialRef.current,
      foreground: foregroundRef.current,
      overlay: overlayRef.current,
    };
    const revision = ++epoch.current;
    if (present && !wasPresent.current) {
      source.current = requestedSource.current?.current ?? null;
      capturedSource.current = requestedSource.current?.snapshot;
      returnSource.current = requestedSource.current?.returnTarget ?? null;
      if (requestedSource.current) {
        delete requestedSource.current.snapshot;
        delete requestedSource.current.returnTarget;
      }
    }
    wasPresent.current = present;
    if (!present && lastBox.current?.height) {
      element.style.height = `${lastBox.current.height}px`;
      element.style.width = `${Math.min(lastBox.current.width, innerWidth - 32)}px`;
    } else {
      element.style.removeProperty("height");
      element.style.removeProperty("width");
    }
    const box = element.getBoundingClientRect();
    // Resolve the explicit return anchor before every exit path, including
    // immediate settlement when native animation cannot run.
    const presentationSource =
      !present && !dialogOriginGeometry(source.current, box).sourced
        ? returnSource.current
        : source.current;
    const liveOrigin = dialogOriginGeometry(presentationSource, box);
    element.dataset.motionOrigin = liveOrigin.sourced ? "trigger" : "neutral";
    const duration =
      motionDuration(present ? "dialog-enter" : "dialog-exit") * 1000;
    let handle: ReturnType<typeof runDialogPresentation> | null = null;
    const settle = () => {
      settleContentResize();
      handle?.cancel(false);
      if (present) settleDialogPlanes(planes);
      else {
        planes.material.style.opacity = "0";
        planes.foreground.style.opacity = "0";
        planes.overlay.style.opacity = "0";
      }
      // Parent Presence registers exit keys after child layout effects.
      void Promise.resolve().then(() => {
        if (revision !== epoch.current) return;
        if (present && box.width && box.height) setSettledPhase(phase);
        else if (!present) remove.current?.();
      });
    };
    if (
      reduce ||
      document.hidden ||
      !box.width ||
      !box.height ||
      !nativeAnimation ||
      !duration
    ) {
      settle();
      return () => {
        epoch.current += 1;
      };
    }
    try {
      handle = runDialogPresentation({
        planes,
        source: presentationSource,
        windowNode: element,
        entering: present,
        first: !hasAnimated.current,
        duration,
        capturedOrigin: capturedSource.current,
      });
      hasAnimated.current = true;
      capturedSource.current = undefined;
      originRetargetRef.current = () => {
        try {
          handle?.retarget();
        } catch {
          settle();
        }
      };
      void handle.contentFinished.then((completed) => {
        if (completed && !present && revision === epoch.current)
          setRetiredPhase(phase);
      });
      void handle.finished.then((completed) => {
        if (completed && revision === epoch.current) settle();
      });
    } catch {
      console.warn("Dialog animation unavailable; settled without motion");
      settle();
    }
    resizeTransition.current = settle;
    const visibility = () => {
      if (document.hidden) settle();
    };
    window.addEventListener("resize", settle);
    document.addEventListener("visibilitychange", visibility);
    return () => {
      epoch.current += 1;
      freezeContentResize();
      handle?.cancel();
      resizeTransition.current = null;
      originRetargetRef.current = null;
      window.removeEventListener("resize", settle);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, [
    present,
    reduce,
    mountVersion,
    nativeAnimation,
    phase,
    settleContentResize,
    freezeContentResize,
  ]);

  useLayoutEffect(() => {
    const root = contentRef.current;
    if (!interactive || !root || document.activeElement !== root) return;
    const target =
      initialFocusRef?.current ??
      root.querySelector<HTMLElement>(
        'input:not(:disabled), textarea:not(:disabled), select:not(:disabled), button:not(:disabled), [tabindex="0"]',
      );
    target?.focus({ preventScroll: true });
  }, [interactive, initialFocusRef]);

  return (
    <DialogPrimitive.Root
      open
      onOpenChange={(next) => {
        if (present) onOpenChange(next);
      }}
    >
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          ref={overlayRef}
          className="fy-control-dialog-overlay"
        />
        <DialogPrimitive.Content
          ref={setContent}
          data-motion-phase={present ? "open" : "exit"}
          data-motion-settled={
            present && settled && !resizing ? "true" : undefined
          }
          data-content-motion={present && resizing ? "resizing" : undefined}
          className={classNames(
            "fy-control-dialog",
            size !== "standard" && `fy-control-dialog-${size}`,
          )}
          {...(!description ? { "aria-describedby": undefined } : {})}
          onPointerDownCapture={(event) => {
            if (!interactive) {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
          onClickCapture={(event) => {
            if (!interactive) {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
          onKeyDownCapture={(event) => {
            if (!interactive && event.key !== "Tab" && event.key !== "Escape") {
              event.preventDefault();
              event.stopPropagation();
            }
          }}
          onOpenAutoFocus={(event) => {
            if (restoreFrameRef.current !== null)
              window.cancelAnimationFrame(restoreFrameRef.current);
            const focused =
              returnSource.current ??
              originRef?.returnTarget ??
              originRef?.current ??
              document.activeElement;
            if (focused instanceof HTMLElement && focused !== document.body)
              restoreFocusRef.current = focused;
            if (nativeAnimation && !reduce) {
              event.preventDefault();
              contentRef.current?.focus({ preventScroll: true });
            } else if (
              initialFocusRef?.current &&
              !initialFocusRef.current.matches(":disabled")
            ) {
              event.preventDefault();
              initialFocusRef.current.focus();
            }
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            const origin = restoreFocusRef.current;
            const focusAtClose = document.activeElement;
            if (restoreFrameRef.current !== null)
              window.cancelAnimationFrame(restoreFrameRef.current);
            restoreFrameRef.current = window.requestAnimationFrame(() => {
              restoreFrameRef.current = null;
              // A user may already have focused an editor after dismissal.
              // An older decorative close must not take that explicit focus back.
              if (
                document.activeElement !== focusAtClose &&
                document.activeElement !== document.body
              )
                return;
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
          <div ref={materialRef} className="fy-dialog-material" aria-hidden>
            <div
              ref={sourceMaterialRef}
              className="fy-dialog-source-material"
            />
            <div ref={targetMaterialRef} className="fy-dialog-target-material">
              <FrostedSurface enhanced={settled} />
            </div>
          </div>
          <div
            ref={foregroundRef}
            className="fy-dialog-foreground"
            {...(!interactive ? { inert: "", "aria-hidden": true } : {})}
          >
            <div
              className="fy-control-dialog-content"
              ref={bodyPresentationRef}
            >
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
              {keepBody && children != null && (
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
  onConfirm: MouseEventHandler<HTMLButtonElement>;
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
