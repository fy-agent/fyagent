import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { errorMessage } from "../../shared/features/helpers";
import type {
  ManagedAuthLoginMethod,
  ManagedAuthLoginSessionSnapshot,
  ManagedAuthPort,
  StartManagedAuthLoginRequest,
} from "../../shared/features/managed-auth";

const POLL_INTERVAL_MS = 1_000;

export function useManagedAuthLoginSession({
  port,
  active,
  onTerminal,
}: {
  port: ManagedAuthPort;
  active: boolean;
  onTerminal?: (snapshot: ManagedAuthLoginSessionSnapshot) => void;
}) {
  const [snapshot, setSnapshot] =
    useState<ManagedAuthLoginSessionSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const terminalHandled = useRef<string | null>(null);
  const lastRequest = useRef<StartManagedAuthLoginRequest | null>(null);

  const acceptSnapshot = useCallback(
    (next: ManagedAuthLoginSessionSnapshot) => {
      setSnapshot(next);
      setError(null);
      if (!next.terminal) return;
      const key = `${next.sessionId}:${next.stage}`;
      if (terminalHandled.current === key) return;
      terminalHandled.current = key;
      onTerminal?.(next);
    },
    [onTerminal],
  );

  useEffect(() => {
    if (!active || snapshot === null || snapshot.terminal) return;
    let disposed = false;
    let timer: number | null = null;

    const poll = async () => {
      try {
        const next = await port.getLoginSession(snapshot.sessionId);
        if (!disposed) acceptSnapshot(next);
      } catch (cause) {
        if (!disposed) setError(errorMessage(cause));
      } finally {
        if (!disposed) timer = window.setTimeout(poll, POLL_INTERVAL_MS);
      }
    };

    timer = window.setTimeout(poll, POLL_INTERVAL_MS);
    return () => {
      disposed = true;
      if (timer !== null) window.clearTimeout(timer);
    };
  }, [acceptSnapshot, active, port, snapshot]);

  const start = useCallback(
    async (request: StartManagedAuthLoginRequest) => {
      setSubmitting(true);
      setError(null);
      lastRequest.current = request;
      terminalHandled.current = null;
      try {
        const next = await port.startLogin(request);
        acceptSnapshot(next);
        return next;
      } catch (cause) {
        setError(errorMessage(cause));
        return null;
      } finally {
        setSubmitting(false);
      }
    },
    [acceptSnapshot, port],
  );

  const resume = useCallback(
    (next: ManagedAuthLoginSessionSnapshot) => {
      if (
        snapshot?.sessionId === next.sessionId &&
        snapshot.stage === next.stage
      ) {
        return;
      }
      acceptSnapshot(next);
    },
    [acceptSnapshot, snapshot],
  );

  const invokeSessionAction = useCallback(
    async (
      action: () => Promise<ManagedAuthLoginSessionSnapshot>,
    ): Promise<ManagedAuthLoginSessionSnapshot | null> => {
      setSubmitting(true);
      setError(null);
      try {
        const next = await action();
        acceptSnapshot(next);
        return next;
      } catch (cause) {
        setError(errorMessage(cause));
        return null;
      } finally {
        setSubmitting(false);
      }
    },
    [acceptSnapshot],
  );

  const cancel = useCallback(async () => {
    if (!snapshot?.canCancel) return null;
    return invokeSessionAction(() => port.cancelLogin(snapshot.sessionId));
  }, [invokeSessionAction, port, snapshot]);

  const reopen = useCallback(async () => {
    if (!snapshot || snapshot.terminal) return null;
    return invokeSessionAction(() => port.reopenLogin(snapshot.sessionId));
  }, [invokeSessionAction, port, snapshot]);

  const switchMethod = useCallback(
    async (method: ManagedAuthLoginMethod) => {
      if (!snapshot || snapshot.terminal) return null;
      return invokeSessionAction(() =>
        port.switchLoginMethod(snapshot.sessionId, method),
      );
    },
    [invokeSessionAction, port, snapshot],
  );

  const retry = useCallback(async () => {
    const request =
      lastRequest.current ??
      (snapshot
        ? {
            provider: snapshot.provider,
            purpose: snapshot.purpose,
            consumer: snapshot.consumer,
            method: snapshot.method,
            accountId: snapshot.accountId,
          }
        : null);
    if (!request) return null;
    return start(request);
  }, [snapshot, start]);

  const reset = useCallback(() => {
    if (snapshot && !snapshot.terminal) return;
    setSnapshot(null);
    setError(null);
    terminalHandled.current = null;
  }, [snapshot]);

  return useMemo(
    () => ({
      snapshot,
      error,
      submitting,
      busy: submitting || (snapshot !== null && !snapshot.terminal),
      start,
      resume,
      cancel,
      reopen,
      switchMethod,
      retry,
      reset,
    }),
    [
      cancel,
      error,
      reopen,
      reset,
      resume,
      retry,
      snapshot,
      start,
      submitting,
      switchMethod,
    ],
  );
}

export type ManagedAuthLoginController = ReturnType<
  typeof useManagedAuthLoginSession
>;
