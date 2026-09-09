import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";

import {
  AGENT_CATALOG_IDS,
  type AgentCatalogId,
} from "../../shared/features/directory";
import { HEALTH_STALE_AFTER_MS } from "../../shared/features/health";
import { useFeatures } from "../../shared/features/provider";
import {
  agentHealthQueryOptions,
  useAgentHealthSnapshots,
} from "../../shared/features/queries";

interface CheckProgress {
  total: number;
  completed: number;
  failed: number;
  current: AgentCatalogId | null;
  state: "running" | "stopping" | "stopped" | "complete";
}
interface CheckRun {
  stopped: boolean;
  done: Promise<void>;
}

export function useHealthChecks(selected: AgentCatalogId, visible: boolean) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const queries = useAgentHealthSnapshots(AGENT_CATALOG_IDS);
  const [progress, setProgress] = useState<CheckProgress | null>(null);
  const [now, setNow] = useState(Date.now);
  const active = useRef(false);
  const currentRun = useRef<CheckRun | null>(null);

  const stop = useCallback(() => {
    if (!currentRun.current) return;
    currentRun.current.stopped = true;
    setProgress((previous) =>
      previous?.state === "running"
        ? { ...previous, state: "stopping" }
        : previous,
    );
  }, []);

  const run = useCallback(
    (ids: readonly AgentCatalogId[]): Promise<void> => {
      if (!active.current || currentRun.current) return Promise.resolve();
      const job: CheckRun = { stopped: false, done: Promise.resolve() };
      currentRun.current = job;
      let completed = 0;
      let failed = 0;
      setProgress({
        total: ids.length,
        completed,
        failed,
        current: ids[0] ?? null,
        state: "running",
      });
      job.done = (async () => {
        for (const agentId of ids) {
          if (job.stopped || !active.current) break;
          setProgress({
            total: ids.length,
            completed,
            failed,
            current: agentId,
            state: "running",
          });
          try {
            await queryClient.fetchQuery({
              ...agentHealthQueryOptions(ports.health, agentId),
              staleTime: 0,
            });
          } catch {
            // Query retains the previous snapshot and failure time for this Agent.
            failed += 1;
          }
          completed += 1;
        }
        if (currentRun.current === job) {
          currentRun.current = null;
          setNow(Date.now());
          setProgress({
            total: ids.length,
            completed,
            failed,
            current: null,
            state: job.stopped ? "stopped" : "complete",
          });
        }
      })();
      return job.done;
    },
    [ports.health, queryClient],
  );

  useEffect(() => {
    active.current = visible;
    let disposed = false;
    // A late read may settle, but route changes revoke further dispatch.
    const pending = currentRun.current;
    if (pending) pending.stopped = true;
    if (visible) {
      void (async () => {
        // Refresh the clock before waiting for the outstanding read. A
        // microtask observes committed visibility without a synchronous
        // effect-state cascade or a timer that waits for the native result.
        await Promise.resolve();
        if (disposed) return;
        setNow(Date.now());
        await pending?.done;
        if (!disposed) await run([selected]);
      })();
    }
    return () => {
      disposed = true;
      active.current = false;
      if (currentRun.current) currentRun.current.stopped = true;
    };
  }, [run, selected, visible]);

  const nextExpiry = Math.min(
    ...queries
      .map((query) =>
        query.data
          ? Date.parse(query.data.checkedAt) + HEALTH_STALE_AFTER_MS
          : Infinity,
      )
      .filter((expiry) => expiry > now),
  );
  useEffect(() => {
    if (!visible || !Number.isFinite(nextExpiry)) return;
    const timer = window.setTimeout(
      () => setNow(Date.now()),
      Math.max(1, nextExpiry - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [nextExpiry, visible]);

  return {
    queries,
    now,
    progress,
    busy: progress?.state === "running" || progress?.state === "stopping",
    refreshSelected: () => run([selected]),
    refreshAll: () => run(AGENT_CATALOG_IDS),
    stop,
  };
}
