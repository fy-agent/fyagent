import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import type {
  DatabaseBackupEntry,
  DatabaseRecoveryPort,
  DatabaseRecoveryReadability,
  DatabaseRestoreOutcome,
} from "../database-recovery";

const MAX_RESTORE_ATTEMPTS = 2;

/** Keep mounted across closes: publication knowledge belongs to the operation,
 * whereas list responses and selections belong to the visible session. */
export function useDatabaseRecoverySession(
  port: DatabaseRecoveryPort,
  active: boolean,
) {
  const session = useMemo(() => ({ active, port }), [active, port]);
  const mounted = useRef(false);
  const admitted = useRef(false);
  const readGeneration = useRef(0);
  const checkGeneration = useRef(0);
  const checkLock = useRef(false);
  const writeLock = useRef(false);
  const [backups, setBackups] = useState<DatabaseBackupEntry[]>([]);
  const [reading, setReading] = useState(false);
  const [loadedSession, setLoadedSession] = useState<typeof session | null>(
    null,
  );
  const [listError, setListError] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const [attempts, setAttempts] = useState(0);
  const [operationFilename, setOperationFilename] = useState<string | null>(
    null,
  );
  const [outcome, setOutcome] = useState<DatabaseRestoreOutcome | null>(null);
  const [unconfirmed, setUnconfirmed] = useState(false);
  const [readability, setReadability] =
    useState<DatabaseRecoveryReadability | null>(null);
  const [readabilitySession, setReadabilitySession] = useState<
    typeof session | null
  >(null);
  const [checking, setChecking] = useState(false);

  useLayoutEffect(() => {
    admitted.current = active;
    return () => {
      admitted.current = false;
      readGeneration.current += 1;
      checkGeneration.current += 1;
    };
  }, [active, port]);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      readGeneration.current += 1;
      checkGeneration.current += 1;
    };
  }, []);

  const readList = useCallback(() => {
    if (!admitted.current || writeLock.current) return;
    const generation = ++readGeneration.current;
    return port
      .list()
      .then(
        (entries) => {
          if (
            !mounted.current ||
            !admitted.current ||
            generation !== readGeneration.current
          )
            return;
          setBackups(entries);
          setListError(false);
          setSelected(null);
          setLoadedSession(session);
        },
        () => {
          if (
            !mounted.current ||
            !admitted.current ||
            generation !== readGeneration.current
          )
            return;
          setBackups([]);
          setListError(true);
          setSelected(null);
          setLoadedSession(session);
        },
      )
      .finally(() => {
        if (
          mounted.current &&
          admitted.current &&
          generation === readGeneration.current
        )
          setReading(false);
      });
  }, [port, session]);

  useEffect(() => {
    // Opening derives its pending state from the fresh session identity; only
    // the native response callback updates the observed list.
    if (active && !restoring && loadedSession !== session) void readList();
  }, [active, readList, restoring, loadedSession, session]);

  const refresh = () => {
    if (!admitted.current || writeLock.current) return;
    setReading(true);
    setListError(false);
    setSelected(null);
    return readList();
  };

  const loading = active && (reading || loadedSession !== session);

  const writesBlocked =
    unconfirmed || (outcome !== null && outcome.publication !== "notCommitted");
  const canRestore =
    active &&
    !loading &&
    !listError &&
    !restoring &&
    !writesBlocked &&
    attempts < MAX_RESTORE_ATTEMPTS &&
    selected !== null &&
    backups.some((entry) => entry.filename === selected);

  const restore = async () => {
    if (
      !admitted.current ||
      !canRestore ||
      selected === null ||
      writeLock.current ||
      checkLock.current
    )
      return;
    const filename = selected;
    writeLock.current = true;
    readGeneration.current += 1;
    checkGeneration.current += 1;
    setRestoring(true);
    setAttempts((current) => current + 1);
    setOperationFilename(filename);
    setOutcome(null);
    setUnconfirmed(true);
    setReadability(null);
    try {
      const result = await port.restore(filename);
      if (!mounted.current) return;
      setOutcome(result);
      setUnconfirmed(false);
    } catch {
      // Never infer a rollback from a rejected IPC or an unreadable response.
      if (mounted.current) setUnconfirmed(true);
    } finally {
      writeLock.current = false;
      if (mounted.current) setRestoring(false);
    }
  };

  const revoke = () => {
    admitted.current = false;
    readGeneration.current += 1;
    checkGeneration.current += 1;
    setSelected(null);
  };

  const checkReadability = async () => {
    if (!admitted.current || writeLock.current || checkLock.current) return;
    const generation = ++checkGeneration.current;
    checkLock.current = true;
    setChecking(true);
    setReadability(null);
    try {
      const result = await port.checkReadability();
      if (
        mounted.current &&
        admitted.current &&
        generation === checkGeneration.current
      ) {
        setReadability(result);
        setReadabilitySession(session);
      }
    } catch {
      if (
        mounted.current &&
        admitted.current &&
        generation === checkGeneration.current
      ) {
        setReadability({ contractVersion: 1, state: "unavailable" });
        setReadabilitySession(session);
      }
    } finally {
      checkLock.current = false;
      if (mounted.current) setChecking(false);
    }
  };

  return {
    backups: loadedSession === session ? backups : [],
    loading,
    listError,
    selected,
    select: (filename: string) => {
      if (
        admitted.current &&
        !loading &&
        !restoring &&
        !writesBlocked &&
        backups.some((entry) => entry.filename === filename)
      ) {
        setSelected(filename);
      }
    },
    restoring,
    outcome,
    unconfirmed,
    operationFilename,
    canRestore: canRestore && !checking,
    writesBlocked,
    checking,
    readability: readabilitySession === session ? readability : null,
    checkReadability,
    retryExhausted: attempts >= MAX_RESTORE_ATTEMPTS,
    refresh,
    restore,
    revoke,
  };
}
