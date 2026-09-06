import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MCP_TARGETS } from "@/shared/features/types";
import { BulkAssignmentPanel } from "@/shared/ui/BulkAssignmentPanel";
import type { DialogOriginRef } from "@/shared/ui/dialogOrigin";

describe("BulkAssignmentPanel", () => {
  it("keeps the closed target order and forwards one exact action and source", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    const origin: DialogOriginRef = { current: null };
    const { container, rerender } = render(
      <BulkAssignmentPanel
        targets={MCP_TARGETS}
        onToggle={onToggle}
        dialogOriginRef={origin}
      />,
    );
    expect(
      [...container.querySelectorAll(".fy-feature-bulk-label")].map(
        (node) => node.textContent,
      ),
    ).toEqual(MCP_TARGETS.map((target) => target.label));
    expect(screen.getAllByRole("button")).toHaveLength(14);
    const row = screen
      .getByText("WorkBuddy")
      .closest(".fy-feature-bulk-row") as HTMLElement;
    const enable = within(row).getByRole("button", { name: "全开" });
    await user.click(enable);
    expect(onToggle).toHaveBeenCalledExactlyOnceWith("workbuddy", true);
    expect(origin.current).toBe(enable);
    await user.click(within(row).getByRole("button", { name: "全关" }));
    expect(onToggle).toHaveBeenLastCalledWith("workbuddy", false);
    rerender(
      <BulkAssignmentPanel
        targets={MCP_TARGETS}
        onToggle={onToggle}
        disabled
        dialogOriginRef={origin}
      />,
    );
    for (const button of screen.getAllByRole("button"))
      expect(button).toBeDisabled();
    await user.click(enable);
    expect(onToggle).toHaveBeenCalledTimes(2);
  });
});
