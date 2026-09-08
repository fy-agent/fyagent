import { useCallback, useEffect, useRef, useState } from "react";

import type {
  AgentAuthPort,
  AgentAuthSessionSnapshot,
  StartAgentAuthSessionRequest,
} from "../../shared/features/agent-auth";

const POLL_INTERVAL_MS = 750;

export function isAgentAuthSessionTerminal(
  snapshot: AgentAuthSessionSnapshot | null,
): boolean {
  return (
    snapshot !== null &&
    [
      "verified",
      "handoff_complete",
      "failed",
      "cancelled",
      "timed_out",
    ].includes(snapshot.stage)
  );
}

export function useAgentAuthSession({
  agentId,
  port,
  enabled = true,
  onTerminal,
}: {
  agentId: StartAgentAuthSessionRequest["agentId"];
  port: AgentAuthPort;
  enabled?: boolean;
  onTerminal?: (snapshot: AgentAuthSessionSnapshot) => void;
}) {
  const [snapshot, setSnapshot] = useState<AgentAuthSessionSnapshot | null>(
    null,
  );
  const [submitting, setSubmitting] = useState(false);
  const [recovering, setRecovering] = useState(enabled);
  const [error, setError] = useState<unknown>(null);
  const terminalCallback = useRef(onTerminal);

  useEffect(() => {
    terminalCallback.current = onTerminal;
  }, [onTerminal]);

  useEffect(() => {
    if (!enabled) return;
    let active = true;
    void port.getActiveSession(agentId).then(
      (next) => {
        if (!active) return;
        setSnapshot(next);
        setError(null);
        setRecovering(false);
      },
      (nextError) => {
        if (!active) return;
        setError(nextError);
        setRecovering(false);
      },
    );
    return () => {
      active = false;
    };
  }, [agentId, enabled, port]);

  useEffect(() => {
    if (!snapshot || isAgentAuthSessionTerminal(snapshot)) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const poll = async () => {
      try {
        const next = await port.getSession(snapshot.sessionId);
        if (!active) return;
        setSnapshot(next);
        setError(null);
        if (isAgentAuthSessionTerminal(next)) {
          terminalCallback.current?.(next);
          return;
        }
      } catch (nextError) {
        if (!active) return;
        setError(nextError);
      }
      if (active) timer = setTimeout(poll, POLL_INTERVAL_MS);
    };
    timer = setTimeout(poll, POLL_INTERVAL_MS);
    return () => {
      active = false;
      if (timer) clearTimeout(timer);
    };
  }, [port, snapshot]);

  const start = useCallback(
    async (request: StartAgentAuthSessionRequest) => {
      setSubmitting(true);
      setError(null);
      try {
        const next = await port.startSession(request);
        setSnapshot(next);
        if (isAgentAuthSessionTerminal(next)) terminalCallback.current?.(next);
        return next;
      } catch (nextError) {
        setError(nextError);
        return null;
      } finally {
        setSubmitting(false);
      }
    },
    [port],
  );

  const stopWaiting = useCallback(async () => {
    if (!snapshot?.canStopWaiting) return null;
    setSubmitting(true);
    setError(null);
    try {
      const next = await port.stopWaiting(snapshot.sessionId);
      setSnapshot(next);
      terminalCallback.current?.(next);
      return next;
    } catch (nextError) {
      setError(nextError);
      return null;
    } finally {
      setSubmitting(false);
    }
  }, [port, snapshot]);

  const resetTerminal = useCallback(() => {
    setSnapshot((current) =>
      isAgentAuthSessionTerminal(current) ? null : current,
    );
    setError(null);
  }, []);

  return {
    snapshot,
    error,
    submitting,
    recovering,
    busy:
      (enabled && recovering) ||
      submitting ||
      (snapshot !== null && !isAgentAuthSessionTerminal(snapshot)),
    start,
    stopWaiting,
    resetTerminal,
  };
}
