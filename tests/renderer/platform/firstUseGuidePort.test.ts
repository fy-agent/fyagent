import { describe, expect, it, vi } from "vitest";

import { parseFirstUseGuideState } from "@/shared/features/first-use-guide";
import { createSimpleFeaturePorts } from "@/shared/platform/tauri/feature-ports/simple";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("first-use guide native boundary", () => {
  it.each(["pending", "dismissed"])("accepts %s", (state) => {
    expect(parseFirstUseGuideState(state)).toBe(state);
  });

  it.each([
    null,
    undefined,
    true,
    false,
    0,
    "",
    "completed",
    {},
    { state: "pending" },
    [],
  ])("rejects unknown wire input %j", (value) =>
    expect(() => parseFirstUseGuideState(value)).toThrow(),
  );

  it("uses parameter-free commands instead of resubmitting settings", async () => {
    invoke.mockResolvedValueOnce("pending").mockResolvedValueOnce("dismissed");
    const { settings } = createSimpleFeaturePorts();
    await expect(settings.getFirstUseGuideState()).resolves.toBe("pending");
    await expect(settings.dismissFirstUseGuide()).resolves.toBe("dismissed");
    expect(invoke.mock.calls).toEqual([
      ["get_first_use_guide_state"],
      ["dismiss_first_use_guide"],
    ]);
  });

  it("does not accept a pending write acknowledgement as completion", async () => {
    invoke.mockResolvedValueOnce("pending");
    await expect(
      createSimpleFeaturePorts().settings.dismissFirstUseGuide(),
    ).rejects.toThrow("not dismissed");
  });
});
