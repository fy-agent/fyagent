import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { createFeaturePorts } from "../platform/features";
import { errorMessage } from "./helpers";
import type { FeaturePorts } from "./ports";
import type { SkillTargetId } from "./types";

interface ToastMessage {
  id: number;
  tone: "success" | "error" | "info";
  title: string;
  description?: string;
}

interface FeatureContextValue {
  ports: FeaturePorts;
  installTarget: SkillTargetId;
  setInstallTarget: (target: SkillTargetId) => void;
  notify: (message: Omit<ToastMessage, "id">) => void;
}

export interface OpenExternalOptions {
  errorTitle?: string;
}

interface ExternalOpenContextValue {
  openExternal: (url: string, options?: OpenExternalOptions) => Promise<void>;
  openingUrl: string | null;
}

const FeatureContext = createContext<FeatureContextValue | null>(null);
const ExternalOpenContext = createContext<ExternalOpenContextValue | null>(
  null,
);

function createFeatureQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { staleTime: 15_000, retry: 1, refetchOnWindowFocus: false },
      mutations: { retry: 0 },
    },
  });
}

export function FeatureProvider({
  children,
  ports: injectedPorts,
}: {
  children: ReactNode;
  ports?: FeaturePorts;
}) {
  const ports = useMemo(
    () => injectedPorts ?? createFeaturePorts(),
    [injectedPorts],
  );
  const [queryClient] = useState(createFeatureQueryClient);
  const [installTarget, setInstallTarget] = useState<SkillTargetId>("claude");
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const notify = useCallback((message: Omit<ToastMessage, "id">) => {
    const id = Date.now() + Math.random();
    setToasts((current) => [...current, { ...message, id }]);
    window.setTimeout(
      () => setToasts((current) => current.filter((toast) => toast.id !== id)),
      4200,
    );
  }, []);
  const value = useMemo(
    () => ({ ports, installTarget, setInstallTarget, notify }),
    [installTarget, notify, ports],
  );
  const openLock = useRef(false);
  const [openingUrl, setOpeningUrl] = useState<string | null>(null);
  const openExternal = useCallback(
    async (url: string, options?: OpenExternalOptions) => {
      if (openLock.current) return;
      openLock.current = true;
      setOpeningUrl(url);
      try {
        await ports.settings.openExternal(url);
      } catch (error) {
        notify({
          tone: "error",
          title: options?.errorTitle ?? "无法打开链接",
          description: errorMessage(error),
        });
      } finally {
        openLock.current = false;
        setOpeningUrl(null);
      }
    },
    [notify, ports.settings],
  );
  const openValue = useMemo(
    () => ({ openExternal, openingUrl }),
    [openExternal, openingUrl],
  );

  return (
    <QueryClientProvider client={queryClient}>
      <FeatureContext.Provider value={value}>
        <ExternalOpenContext.Provider value={openValue}>
          {children}
          <div className="fy-toast-host" aria-live="polite" aria-atomic="false">
            {toasts.map((toast) => (
              <div
                key={toast.id}
                className={`fy-toast fy-toast-${toast.tone}`}
                role="status"
              >
                <strong>{toast.title}</strong>
                {toast.description && <span>{toast.description}</span>}
              </div>
            ))}
          </div>
        </ExternalOpenContext.Provider>
      </FeatureContext.Provider>
    </QueryClientProvider>
  );
}

export function useFeatures(): FeatureContextValue {
  const context = useContext(FeatureContext);
  if (!context)
    throw new Error("useFeatures must be used within FeatureProvider");
  return context;
}

export function useOpenExternal(): ExternalOpenContextValue {
  const context = useContext(ExternalOpenContext);
  if (!context)
    throw new Error("useOpenExternal must be used within FeatureProvider");
  return context;
}
