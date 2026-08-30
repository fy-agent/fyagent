import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import {
  agentReturnPathFromLocationState,
  createAgentReturnLocationState,
  type AgentSection,
} from "./agent-navigation";
import type { AgentCatalogId } from "./directory";

type AgentReturnNavigationContextValue = Readonly<{
  returnPath: string | null;
  remember: (agentId: AgentCatalogId, section: AgentSection) => void;
  clear: () => void;
}>;

const unavailableContext: AgentReturnNavigationContextValue = {
  returnPath: null,
  remember: () => undefined,
  clear: () => undefined,
};

const AgentReturnNavigationContext =
  createContext<AgentReturnNavigationContextValue>(unavailableContext);

export function AgentReturnNavigationProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [returnPath, setReturnPath] = useState<string | null>(null);
  const remember = useCallback(
    (agentId: AgentCatalogId, section: AgentSection) => {
      setReturnPath(
        agentReturnPathFromLocationState(
          createAgentReturnLocationState(agentId, section),
        ),
      );
    },
    [],
  );
  const clear = useCallback(() => setReturnPath(null), []);
  const value = useMemo(
    () => ({ returnPath, remember, clear }),
    [clear, remember, returnPath],
  );

  return (
    <AgentReturnNavigationContext.Provider value={value}>
      {children}
    </AgentReturnNavigationContext.Provider>
  );
}

export function useAgentReturnNavigation(): AgentReturnNavigationContextValue {
  return useContext(AgentReturnNavigationContext);
}
