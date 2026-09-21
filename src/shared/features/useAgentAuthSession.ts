import { useCallback, useEffect, useRef, useState } from "react";

import type {
  AgentAuthPort,
  AgentAuthSessionSnapshot,
  StartAgentAuthSessionRequest,
} from "./agent-auth";

const POLL_INTERVAL_MS = 750;
const unavailable = () => new Error("Agent auth session is unavailable");

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
  const snapshotRef = useRef(snapshot);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  const [recovering, setRecovering] = useState(enabled);
  const recovered = useRef(false);
  const [recoveryReady, setRecoveryReady] = useState(false);
  const [recoveryAttempt, setRecoveryAttempt] = useState(0);
  const [scope, setScope] = useState({
    agentId,
    port,
    enabled,
    recoveryAttempt,
  });
  const [error, setError] = useState<unknown>(null);
  const generation = useRef(0);
  const lastTerminal = useRef<string | null>(null);
  const terminalCallback = useRef(onTerminal);

  // Reset presentation before committing a changed owner/visibility scope.
  // Effects below synchronize only the native observation and ref admission.
  if (
    scope.agentId !== agentId ||
    scope.port !== port ||
    scope.enabled !== enabled ||
    scope.recoveryAttempt !== recoveryAttempt
  ) {
    setScope({ agentId, port, enabled, recoveryAttempt });
    setRecoveryReady(false);
    setRecovering(enabled);
    setSubmitting(false);
    if (snapshot && snapshot.agentId !== agentId) setSnapshot(null);
  }

  useEffect(() => {
    terminalCallback.current = onTerminal;
  }, [onTerminal]);

  const accept = useCallback((next: AgentAuthSessionSnapshot | null) => {
    snapshotRef.current = next;
    setSnapshot(next);
    setError(null);
    if (
      next &&
      isAgentAuthSessionTerminal(next) &&
      lastTerminal.current !== next.sessionId
    ) {
      lastTerminal.current = next.sessionId;
      terminalCallback.current?.(next);
    }
  }, []);

  useEffect(() => {
    const scope = ++generation.current;
    recovered.current = false;
    submittingRef.current = false;
    if (!enabled) return;
    if (snapshotRef.current?.agentId !== agentId) snapshotRef.current = null;
    void port
      .getActiveSession(agentId)
      .then(async (activeSession) => {
        if (generation.current !== scope) return;
        let next = activeSession;
        const retained = snapshotRef.current;
        // Native active lookup intentionally omits terminal sessions. Recover
        // our known session once if it finished while polling was hidden.
        if (!next && retained && !isAgentAuthSessionTerminal(retained)) {
          next = await port.getSession(retained.sessionId);
          if (generation.current !== scope) return;
          if (
            next.sessionId !== retained.sessionId ||
            next.intent !== retained.intent
          )
            throw unavailable();
        }
        if (next && next.agentId !== agentId) throw unavailable();
        // A hidden terminal result remains reviewable when there is no active
        // native session. A new active session always takes precedence.
        accept(
          next ??
            (isAgentAuthSessionTerminal(snapshotRef.current)
              ? snapshotRef.current
              : null),
        );
        recovered.current = true;
        setRecoveryReady(true);
        setRecovering(false);
      })
      .catch((nextError: unknown) => {
        if (generation.current !== scope) return;
        setError(nextError);
        setRecovering(false);
      });
    return () => {
      generation.current += 1;
    };
  }, [accept, agentId, enabled, port, recoveryAttempt]);

  useEffect(() => {
    if (
      !enabled ||
      !recovered.current ||
      !snapshot ||
      isAgentAuthSessionTerminal(snapshot)
    )
      return;
    let active = true;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const poll = async () => {
      try {
        const next = await port.getSession(snapshot.sessionId);
        if (!active) return;
        if (
          next.agentId !== agentId ||
          next.sessionId !== snapshot.sessionId ||
          next.intent !== snapshot.intent
        )
          throw unavailable();
        accept(next);
        if (isAgentAuthSessionTerminal(next)) return;
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
  }, [accept, agentId, enabled, port, recovering, snapshot]);

  const start = useCallback(
    async (request: StartAgentAuthSessionRequest) => {
      if (
        !enabled ||
        !recovered.current ||
        submittingRef.current ||
        (snapshotRef.current &&
          !isAgentAuthSessionTerminal(snapshotRef.current))
      )
        return null;
      const scope = generation.current;
      submittingRef.current = true;
      setSubmitting(true);
      setError(null);
      try {
        if (request.agentId !== agentId) throw unavailable();
        const next = await port.startSession(request);
        if (generation.current !== scope) return null;
        if (next.agentId !== agentId || next.intent !== request.intent)
          throw unavailable();
        accept(next);
        return next;
      } catch (nextError) {
        if (generation.current === scope) setError(nextError);
        return null;
      } finally {
        if (generation.current === scope) {
          submittingRef.current = false;
          setSubmitting(false);
        }
      }
    },
    [accept, agentId, enabled, port],
  );

  const stopWaiting = useCallback(async () => {
    const current = snapshotRef.current;
    if (!enabled || submittingRef.current || !current?.canStopWaiting)
      return null;
    const scope = generation.current;
    submittingRef.current = true;
    setSubmitting(true);
    setError(null);
    try {
      const next = await port.stopWaiting(current.sessionId);
      if (generation.current !== scope) return null;
      if (
        next.agentId !== agentId ||
        next.sessionId !== current.sessionId ||
        next.intent !== current.intent
      )
        throw unavailable();
      accept(next);
      return next;
    } catch (nextError) {
      if (generation.current === scope) setError(nextError);
      return null;
    } finally {
      if (generation.current === scope) {
        submittingRef.current = false;
        setSubmitting(false);
      }
    }
  }, [accept, agentId, enabled, port]);

  const resetTerminal = useCallback(() => {
    if (isAgentAuthSessionTerminal(snapshotRef.current)) accept(null);
    setError(null);
  }, [accept]);
  const retryRecovery = useCallback(() => {
    recovered.current = false;
    setRecoveryAttempt((attempt) => attempt + 1);
  }, []);

  return {
    snapshot,
    error,
    submitting,
    recovering,
    busy:
      enabled &&
      (!recoveryReady ||
        submitting ||
        (snapshot !== null && !isAgentAuthSessionTerminal(snapshot))),
    start,
    stopWaiting,
    resetTerminal,
    retryRecovery,
  };
}
