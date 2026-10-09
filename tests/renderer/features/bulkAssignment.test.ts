import { describe, expect, it, vi } from "vitest";
import {
  executeBulkAssignment,
  type BulkAssignmentItem,
  type BulkAssignmentPlan,
} from "@/shared/features/bulk-assignment";

const item = (id: string): BulkAssignmentItem => ({
  id,
  name: id,
  identity: "original",
  apps: { codex: false },
});
const plan = (items: BulkAssignmentItem[]): BulkAssignmentPlan => ({
  target: "codex",
  enabled: true,
  items,
});

describe("bulk assignment preview execution", () => {
  it("does not mutate an already matching assignment", async () => {
    const original = { ...item("a"), apps: { codex: true } };
    const mutate = vi.fn();
    expect(
      (
        await executeBulkAssignment(
          plan([original]),
          async () => [original],
          mutate,
        )
      )[0].status,
    ).toBe("confirmed");
    expect(mutate).not.toHaveBeenCalled();
  });
  it("confirms only exact readback and continues after a false result", async () => {
    const originals = [item("a"), item("b"), item("c")];
    let live = structuredClone(originals);
    const mutate = vi.fn(async (id: string) => {
      if (id === "b") return false;
      live = live.map((entry) =>
        entry.id === id ? { ...entry, apps: { codex: true } } : entry,
      );
      return true;
    });
    const results = await executeBulkAssignment(
      plan(originals),
      async () => live,
      mutate,
    );
    expect(results.map((entry) => entry.status)).toEqual([
      "confirmed",
      "failed",
      "confirmed",
    ]);
    expect(mutate.mock.calls.map(([id]) => id)).toEqual(["a", "b", "c"]);
  });

  it.each(["identity", "apps", "missing"])(
    "rejects whole stale preview before writes: %s",
    async (change) => {
      const originals = [item("a"), item("b")];
      const live =
        change === "missing"
          ? [originals[0]]
          : [
              originals[0],
              {
                ...originals[1],
                ...(change === "identity"
                  ? { identity: "changed" }
                  : { apps: { codex: true } }),
              },
            ];
      const mutate = vi.fn();
      const results = await executeBulkAssignment(
        plan(originals),
        async () => live,
        mutate,
      );
      expect(results.map((entry) => entry.status)).toEqual(["drift", "drift"]);
      expect(mutate).not.toHaveBeenCalled();
    },
  );

  it("does not write linked target but permits an ordinary target", async () => {
    const originals = [
      { ...item("a"), readOnlyTargets: ["codex"] },
      { ...item("b"), readOnlyTargets: ["claude"] },
    ];
    let live = structuredClone(originals);
    const mutate = vi.fn(async (id: string) => {
      live = live.map((entry) =>
        entry.id === id ? { ...entry, apps: { codex: true } } : entry,
      );
    });
    const results = await executeBulkAssignment(
      plan(originals),
      async () => live,
      mutate,
    );
    expect(results.map((entry) => entry.status)).toEqual([
      "read_only",
      "confirmed",
    ]);
    expect(mutate).toHaveBeenCalledExactlyOnceWith("b", "codex", true);
  });

  it("reports thrown partial writes as failed even if reread shows changed state", async () => {
    const original = item("a");
    let live = [original];
    const mutate = vi.fn(async () => {
      live = [{ ...original, apps: { codex: true } }];
      throw new Error("private native detail");
    });
    expect(
      (
        await executeBulkAssignment(plan([original]), async () => live, mutate)
      )[0].status,
    ).toBe("failed");
  });

  it("rejects void command acknowledgement when readback lacks the row", async () => {
    const original = item("a");
    let reads = 0;
    const read = vi.fn(async () => (++reads < 3 ? [original] : []));
    expect(
      (
        await executeBulkAssignment(
          plan([original]),
          read,
          vi.fn(async () => {}),
        )
      )[0].status,
    ).toBe("failed");
  });

  it("detects per-item drift after another item completed", async () => {
    const originals = [item("a"), item("b")];
    let live = structuredClone(originals);
    const mutate = vi.fn(async () => {
      live = [
        { ...originals[0], apps: { codex: true } },
        { ...originals[1], identity: "external change" },
      ];
    });
    const results = await executeBulkAssignment(
      plan(originals),
      async () => live,
      mutate,
    );
    expect(results.map((entry) => entry.status)).toEqual([
      "confirmed",
      "drift",
    ]);
    expect(mutate).toHaveBeenCalledTimes(1);
  });
  it("allows reading a linked source when the selected target is ordinary", async () => {
    const original = {
      ...item("a"),
      readOnly: true,
      readOnlyTargets: ["claude"],
    };
    let live = [original];
    const mutate = vi.fn(async () => {
      live = [{ ...original, apps: { codex: true } }];
    });
    expect(
      (
        await executeBulkAssignment(plan([original]), async () => live, mutate)
      )[0].status,
    ).toBe("confirmed");
    expect(mutate).toHaveBeenCalledOnce();
  });

  it.each(["identity", "other_target"])(
    "does not confirm changed readback DTO: %s",
    async (change) => {
      const original = item("a");
      let live: BulkAssignmentItem[] = [original];
      const mutate = vi.fn(async () => {
        live = [
          {
            ...original,
            identity: change === "identity" ? "new source" : original.identity,
            apps:
              change === "other_target"
                ? { codex: true, claude: true }
                : { codex: true },
          },
        ];
      });
      expect(
        (
          await executeBulkAssignment(
            plan([original]),
            async () => live,
            mutate,
          )
        )[0].status,
      ).toBe("failed");
    },
  );

  it("permits native adoption metadata while still checking the resource identity", async () => {
    const original = {
      ...item("a"),
      allowAdoption: true,
      readbackIdentity: "same resource",
    };
    let live: BulkAssignmentItem[] = [original];
    const mutate = vi.fn(async () => {
      live = [
        {
          ...original,
          identity: "native SSOT path/hash/times",
          allowAdoption: false,
          apps: { codex: true },
        },
      ];
    });
    expect(
      (
        await executeBulkAssignment(plan([original]), async () => live, mutate)
      )[0].status,
    ).toBe("confirmed");
  });

  it("invalidates a preview when read-only target observations change", async () => {
    const original = item("a");
    const mutate = vi.fn();
    expect(
      (
        await executeBulkAssignment(
          plan([original]),
          async () => [{ ...original, readOnlyTargets: ["claude"] }],
          mutate,
        )
      )[0].status,
    ).toBe("drift");
    expect(mutate).not.toHaveBeenCalled();
  });
});
