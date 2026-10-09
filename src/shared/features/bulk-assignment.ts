import type { SkillTargetId } from "./directory";

export interface BulkAssignmentItem {
  id: string;
  name: string;
  identity: string;
  /** First assignment may adopt an observed Skill into SSOT. */
  allowAdoption?: boolean;
  readbackIdentity?: string;
  apps: Record<string, boolean | undefined>;
  readOnly?: boolean;
  readOnlyTargets?: readonly string[];
}

export interface BulkAssignmentPlan {
  target: SkillTargetId;
  enabled: boolean;
  items: BulkAssignmentItem[];
}

export interface BulkAssignmentResult {
  id: string;
  name: string;
  status: "confirmed" | "failed" | "drift" | "read_only";
}

function unchanged(before: BulkAssignmentItem, current: BulkAssignmentItem) {
  return (
    before.identity === current.identity &&
    before.readOnly === current.readOnly &&
    JSON.stringify(before.readOnlyTargets) ===
      JSON.stringify(current.readOnlyTargets) &&
    JSON.stringify(before.apps) === JSON.stringify(current.apps)
  );
}

/** Uses existing domain ports; a command acknowledgement is not readback. */
export async function executeBulkAssignment(
  plan: BulkAssignmentPlan,
  read: () => Promise<BulkAssignmentItem[]>,
  mutate: (
    id: string,
    target: SkillTargetId,
    enabled: boolean,
  ) => Promise<boolean | void>,
  onResult?: (result: BulkAssignmentResult) => void,
): Promise<BulkAssignmentResult[]> {
  const results: BulkAssignmentResult[] = [];
  // Fail the whole preview before writing if any selected observation drifted.
  const fresh = await read();
  const drifted = plan.items.some((item) => {
    const current = fresh.find((entry) => entry.id === item.id);
    return !current || !unchanged(item, current);
  });
  for (const item of plan.items) {
    let status: BulkAssignmentResult["status"] = "failed";
    try {
      const current = drifted
        ? undefined
        : (await read()).find((entry) => entry.id === item.id);
      if (drifted || !current || !unchanged(item, current)) status = "drift";
      else if (current.readOnlyTargets?.includes(plan.target))
        status = "read_only";
      else if (current.apps[plan.target] === plan.enabled) status = "confirmed";
      else {
        const accepted = await mutate(item.id, plan.target, plan.enabled);
        const observed = (await read()).find((entry) => entry.id === item.id);
        if (
          accepted !== false &&
          observed &&
          (observed.identity === current.identity ||
            (current.allowAdoption &&
              current.readbackIdentity !== undefined &&
              observed.readbackIdentity === current.readbackIdentity)) &&
          JSON.stringify(observed.apps) ===
            JSON.stringify({ ...current.apps, [plan.target]: plan.enabled })
        )
          status = "confirmed";
      }
    } catch {
      // A partial durable write is still a failure. Best-effort convergence
      // never changes the recorded outcome or fabricates rollback.
      try {
        await read();
      } catch {
        /* The row remains visibly failed. */
      }
    }
    const result = { id: item.id, name: item.name, status };
    results.push(result);
    onResult?.(result);
  }
  return results;
}
