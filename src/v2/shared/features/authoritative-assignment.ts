import { useCallback, useRef, useState } from "react";

export type AuthoritativeRereadResult<TSnapshot> = {
  data: TSnapshot | undefined;
  error: unknown;
};

export type AuthoritativeAssignmentOutcome =
  | { status: "confirmed" }
  | { status: "rejected" }
  | { status: "busy" };

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
      try {
        const accepted = await mutate(itemId, enabled);
        if (accepted === false) {
          throw new Error("assignment rejected");
        }
        const readback = await reread();
        if (readback.error || readValue(readback.data, itemId) !== enabled) {
          throw new Error("assignment readback mismatch");
        }
        return { status: "confirmed" };
      } catch {
        try {
          await reread();
        } catch {
          // The original mutation/readback failure remains authoritative.
        }
        return { status: "rejected" };
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
