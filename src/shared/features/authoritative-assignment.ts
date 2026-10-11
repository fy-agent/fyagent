import { useCallback, useRef, useState } from "react";

export type AuthoritativeRereadResult<TSnapshot> = {
  data: TSnapshot | undefined;
  error: unknown;
};

export type AuthoritativeFailureStage = "mutate" | "readback" | "mismatch";

export type AuthoritativeAssignmentOutcome<TValue = boolean | undefined> =
  | {
      status: "confirmed";
      observedValue?: TValue;
    }
  | {
      status: "rejected";
      stage?: AuthoritativeFailureStage;
      reason?: string;
      observedValue?: TValue;
      error?: unknown;
    }
  | { status: "busy" };

function extractFailureReason(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message.trim()) {
    return err.message;
  }
  if (typeof err === "string" && err.trim()) {
    return err;
  }
  if (err && typeof err === "object") {
    if (
      "message" in err &&
      typeof (err as { message: unknown }).message === "string" &&
      (err as { message: string }).message.trim()
    ) {
      return (err as { message: string }).message;
    }
    if (
      "error" in err &&
      typeof (err as { error: unknown }).error === "string" &&
      (err as { error: string }).error.trim()
    ) {
      return (err as { error: string }).error;
    }
  }
  return fallback;
}

export function useAuthoritativeAssignmentMutation<
  TItemId extends string,
  TSnapshot,
>({
  mutate,
  reread,
  readValue,
}: {
  mutate: (itemId: TItemId, enabled: boolean) => Promise<boolean | void>;
  reread: () => Promise<AuthoritativeRereadResult<TSnapshot>>;
  readValue: (
    snapshot: TSnapshot | undefined,
    itemId: TItemId,
  ) => boolean | undefined;
}) {
  const pendingRef = useRef<TItemId | null>(null);
  const [pendingId, setPendingId] = useState<TItemId | null>(null);

  const run = useCallback(
    async (
      itemId: TItemId,
      enabled: boolean,
    ): Promise<AuthoritativeAssignmentOutcome> => {
      if (pendingRef.current !== null) {
        return { status: "busy" };
      }

      pendingRef.current = itemId;
      setPendingId(itemId);
      let stage: AuthoritativeFailureStage = "mutate";
      let failureReason = "后端拒绝了赋值请求";
      let originalError: unknown = null;

      try {
        let accepted: boolean | void;
        try {
          accepted = await mutate(itemId, enabled);
        } catch (err) {
          stage = "mutate";
          originalError = err;
          failureReason = extractFailureReason(err, "后端拒绝了赋值请求");
          throw err;
        }

        if (accepted === false) {
          stage = "mutate";
          failureReason = "赋值被拒绝";
          throw new Error(failureReason);
        }

        let readback: AuthoritativeRereadResult<TSnapshot>;
        try {
          readback = await reread();
        } catch (err) {
          stage = "readback";
          originalError = err;
          failureReason = extractFailureReason(err, "回读状态失败");
          throw err;
        }

        if (readback.error) {
          stage = "readback";
          originalError = readback.error;
          failureReason = extractFailureReason(readback.error, "回读状态失败");
          throw new Error(failureReason);
        }

        const observed = readValue(readback.data, itemId);
        if (observed !== enabled) {
          stage = "mismatch";
          failureReason = "回读状态与预期不一致";
          throw new Error(failureReason);
        }

        const confirmedOutcome: AuthoritativeAssignmentOutcome = {
          status: "confirmed",
        };
        Object.defineProperties(confirmedOutcome, {
          observedValue: {
            value: observed,
            enumerable: false,
            writable: true,
          },
        });
        return confirmedOutcome;
      } catch {
        console.error(
          `[authoritative-assignment] ${stage} failed for item "${itemId}":`,
          { reason: failureReason, error: originalError },
        );

        let recoveryObserved: boolean | undefined = undefined;
        try {
          const recoveryReadback = await reread();
          if (!recoveryReadback.error && recoveryReadback.data !== undefined) {
            recoveryObserved = readValue(recoveryReadback.data, itemId);
          }
        } catch {
          // The original mutation/readback failure remains authoritative.
        }

        const rejectedOutcome: AuthoritativeAssignmentOutcome = {
          status: "rejected",
        };
        Object.defineProperties(rejectedOutcome, {
          stage: {
            value: stage,
            enumerable: false,
            writable: true,
          },
          reason: {
            value: failureReason,
            enumerable: false,
            writable: true,
          },
          observedValue: {
            value: recoveryObserved,
            enumerable: false,
            writable: true,
          },
          error: {
            value: originalError,
            enumerable: false,
            writable: true,
          },
        });
        return rejectedOutcome;
      } finally {
        pendingRef.current = null;
        setPendingId(null);
      }
    },
    [mutate, readValue, reread],
  );

  return {
    busy: pendingId !== null,
    pendingId,
    run,
  };
}
