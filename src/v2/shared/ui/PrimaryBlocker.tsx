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

type BlockerRule = boolean | BlockerFunction;
type RouteBlocker = ReturnType<typeof useBlocker>;

interface PrimaryBlockerContextValue {
  blocker: RouteBlocker;
  register: (id: string, rule: BlockerRule) => void;
  unregister: (id: string) => void;
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
  const shouldBlock = useCallback<BlockerFunction>((transition) => {
    for (const rule of rules.current.values()) {
      if (typeof rule === "function" ? rule(transition) : rule) return true;
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
    () => ({ blocker, register, unregister }),
    [blocker, register, unregister],
  );

  return (
    <PrimaryBlockerContext.Provider value={value}>
      {children}
    </PrimaryBlockerContext.Provider>
  );
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
