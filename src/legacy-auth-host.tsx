import { QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";
import { ThemeProvider } from "@/components/theme-provider";
import { queryClient } from "@/lib/query";

import "./i18n";

const ROOT_ID = "legacy-auth-root";

let openAuthCenter: (() => void) | null = null;
let mounted = false;

function LegacyAuthHost() {
  const [open, setOpen] = useState(true);

  useEffect(() => {
    openAuthCenter = () => setOpen(true);
    return () => {
      if (openAuthCenter) openAuthCenter = null;
    };
  }, []);

  if (!open) return null;

  return (
    <div
      className="legacy-auth-host"
      role="dialog"
      aria-modal="true"
      aria-label="认证中心"
    >
      <header className="legacy-auth-host-header">
        <div>
          <h1>认证中心</h1>
          <p>
            SuperGrok 扫码在下面的 xAI (Grok OAuth)。不要在这里跑 grok
            login。关掉后回到新界面。
          </p>
        </div>
        <button type="button" onClick={() => setOpen(false)}>
          关闭
        </button>
      </header>
      <AuthCenterPanel />
    </div>
  );
}

function ensureMounted(): void {
  if (mounted) return;
  mounted = true;
  const mount = document.createElement("div");
  mount.id = ROOT_ID;
  document.body.appendChild(mount);
  createRoot(mount).render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="system" storageKey="fyagent-theme">
        <LegacyAuthHost />
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

export function openLegacyAuthCenter(): void {
  ensureMounted();
  openAuthCenter?.();
}
