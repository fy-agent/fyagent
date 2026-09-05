import {
  createContext,
  useCallback,
  useContext,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import { useBlocker, type BlockerFunction } from "react-router-dom";

import { usePersistentVisibility } from "./PersistentSurface";
import type { DialogOriginRef } from "./dialogOrigin";

type BlockerRule = boolean | BlockerFunction;
type RouteBlocker = ReturnType<typeof useBlocker>;

interface PrimaryBlockerContextValue {
  blocker: RouteBlocker;
  register: (id: string, rule: BlockerRule) => void;
  unregister: (id: string) => void;
  originRef: DialogOriginRef;
  captureOrigin: (element: HTMLElement, destination: string) => void;
}

const PrimaryBlockerContext = createContext<PrimaryBlockerContextValue | null>(
  null,
);

const idleBlocker = {
  state: "unblocked",
  location: {
    pathname: "",
    search: "",
    hash: "",
    state: null,
    key: "idle",
  },
} as unknown as RouteBlocker;

export function PrimaryBlockerProvider({ children }: { children: ReactNode }) {
  const rules = useRef(new Map<string, BlockerRule>());
  const originRef = useRef<HTMLElement | null>(null);
  const intent = useRef<{ element: HTMLElement; destination: string } | null>(
    null,
  );
  const captureOrigin = useCallback(
    (element: HTMLElement, destination: string) => {
      intent.current = { element, destination };
    },
    [],
  );
  const shouldBlock = useCallback<BlockerFunction>((transition) => {
    const pending = intent.current;
    intent.current = null;
    originRef.current = null;
    for (const rule of rules.current.values()) {
      if (typeof rule === "function" ? rule(transition) : rule) {
        const destination = `${transition.nextLocation.pathname}${transition.nextLocation.search}`;
        if (pending?.destination === destination)
          originRef.current = pending.element;
        return true;
      }
    }
    return false;
  }, []);
  const blocker = useBlocker(shouldBlock);
  const register = useCallback((id: string, rule: BlockerRule) => {
    rules.current.set(id, rule);
  }, []);
  const unregister = useCallback((id: string) => {
    rules.current.delete(id);
  }, []);
  const value = useMemo(
    () => ({ blocker, register, unregister, originRef, captureOrigin }),
    [blocker, register, unregister, captureOrigin],
  );

  return (
    <PrimaryBlockerContext.Provider value={value}>
      {children}
    </PrimaryBlockerContext.Provider>
  );
}

const ignoreOrigin = () => undefined;

/** A scoped navigation intent, consumed once by the existing blocker. It does
 * not observe global clicks or change route admission/confirmation policy. */
export function usePrimaryNavigationOrigin() {
  return useContext(PrimaryBlockerContext)?.captureOrigin ?? ignoreOrigin;
}

export function usePrimaryBlockerOrigin(): DialogOriginRef {
  const context = useContext(PrimaryBlockerContext);
  if (!context)
    throw new Error("usePrimaryBlockerOrigin requires PrimaryBlockerProvider");
  return context.originRef;
}

export function usePrimaryBlocker(rule: BlockerRule): RouteBlocker {
  const context = useContext(PrimaryBlockerContext);
  const visible = usePersistentVisibility();
  const id = useId();
  const activeRule: BlockerRule = visible ? rule : false;

  useLayoutEffect(() => {
    if (!context) return;
    context.register(id, activeRule);
    return () => context.unregister(id);
  }, [activeRule, context, id]);

  if (!context) {
    throw new Error("usePrimaryBlocker requires PrimaryBlockerProvider");
  }

  return visible ? context.blocker : idleBlocker;
}
