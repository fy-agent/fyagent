import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type MutableRefObject,
  type RefObject,
} from "react";
import { runDialogResize } from "./dialogPresentation";
import { motionDuration } from "./motion";

/** Content layout is distinct from origin presentation and viewport resizing.
 * The fixed dialog's real size changes; text is never scale-projected/copied. */
export function useDialogResize({
  rootRef,
  bodyRef,
  lastBoxRef,
  originSettler,
  present,
  settled,
  reduce,
  nativeAnimation,
  mountVersion,
  presentationKey,
}: {
  rootRef: RefObject<HTMLElement>;
  bodyRef: RefObject<HTMLElement>;
  lastBoxRef: MutableRefObject<DOMRect | null>;
  originSettler: MutableRefObject<(() => void) | null>;
  present: boolean;
  settled: boolean;
  reduce: boolean;
  nativeAnimation: boolean;
  mountVersion: number;
  presentationKey?: string | number;
}) {
  const [resizing, setResizing] = useState(false);
  const active = useRef<ReturnType<typeof runDialogResize> | null>(null);
  const previousKey = useRef(presentationKey);

  const cancel = useCallback(
    (freeze: boolean) => {
      const handle = active.current;
      if (!handle) return;
      active.current = null;
      if (freeze && rootRef.current)
        lastBoxRef.current = rootRef.current.getBoundingClientRect();
      handle.cancel(freeze);
    },
    [lastBoxRef, rootRef],
  );

  const settle = useCallback(() => {
    cancel(false);
    setResizing(false);
    if (rootRef.current)
      lastBoxRef.current = rootRef.current.getBoundingClientRect();
  }, [cancel, lastBoxRef, rootRef]);
  const freeze = useCallback(() => cancel(true), [cancel]);

  useLayoutEffect(() => () => cancel(false), [cancel]);
  useLayoutEffect(() => {
    const root = rootRef.current;
    const body = bodyRef.current;
    if (!root || !body || !present) return;
    const semanticChange = previousKey.current !== presentationKey;
    previousKey.current = presentationKey;
    const measure = (semantic = false) => {
      // ResizeObserver sees our animated size on every frame. Observing that
      // size is not another business change and must not restart the tween.
      if (active.current && !semantic) {
        lastBoxRef.current = root.getBoundingClientRect();
        return;
      }
      const from = active.current
        ? root.getBoundingClientRect()
        : lastBoxRef.current;
      if (active.current) cancel(false);
      const target = root.getBoundingClientRect();
      lastBoxRef.current = target;
      const changed =
        from &&
        (Math.abs(from.width - target.width) > 0.5 ||
          Math.abs(from.height - target.height) > 0.5);
      if (!from || (!changed && !semantic)) return;
      const duration = motionDuration("dialog-resize") * 1000;
      if (
        !settled ||
        reduce ||
        document.hidden ||
        !nativeAnimation ||
        !duration ||
        !target.width ||
        !target.height
      ) {
        settle();
        originSettler.current?.();
        return;
      }
      try {
        const handle = runDialogResize({
          windowNode: root,
          contentNode: body,
          from,
          target,
          duration,
        });
        active.current = handle;
        setResizing(true);
        void handle.finished.then((completed) => {
          if (!completed || active.current !== handle) return;
          lastBoxRef.current = root.getBoundingClientRect();
          cancel(false);
          setResizing(false);
          // Async content may have changed while a fixed animation size was
          // active. Reconcile its natural size once, never with a polling loop.
          measure();
        });
      } catch {
        console.warn(
          "Dialog content animation unavailable; settled without motion",
        );
        settle();
        originSettler.current?.();
      }
    };
    const viewportChanged = () => {
      settle();
      originSettler.current?.();
    };
    measure(semanticChange);
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => measure());
    // Observe intrinsic content, not the size this controller writes. The
    // header/body/action row keep natural height inside the fixed scrollport;
    // observing the animated root would generate an observer feedback loop.
    root
      .querySelectorAll<HTMLElement>(
        ".fy-control-dialog-header, .fy-control-dialog-body, .fy-control-dialog-actions",
      )
      .forEach((node) => observer?.observe(node));
    window.addEventListener("resize", viewportChanged);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", viewportChanged);
    };
  }, [
    present,
    settled,
    reduce,
    nativeAnimation,
    mountVersion,
    presentationKey,
    rootRef,
    bodyRef,
    lastBoxRef,
    originSettler,
    cancel,
    settle,
  ]);

  return { resizing, settle, freeze };
}
