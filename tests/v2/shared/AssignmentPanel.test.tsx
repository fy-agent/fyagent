import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  createMcpAssignments,
  createSkillAssignments,
  MCP_TARGETS,
  SKILL_TARGETS,
} from "@/v2/shared/features/types";
import { AssignmentPanel } from "@/v2/shared/ui/AssignmentPanel";

describe("AssignmentPanel", () => {
  it("renders one seven-switch Skills panel with decorative local target icons", () => {
    const { container } = render(
      <AssignmentPanel
        apps={createSkillAssignments(["claude", "opencode", "qoderwork"])}
        disabled={false}
        labelSuffix="Skill 分配"
        onToggle={vi.fn()}
        targets={SKILL_TARGETS}
      />,
    );

    expect(screen.getAllByRole("heading", { name: "应用分配" })).toHaveLength(
      1,
    );
    expect(screen.getAllByRole("switch")).toHaveLength(7);

    for (const app of SKILL_TARGETS) {
      expect(screen.getByText(app.label)).toBeVisible();
      expect(
        screen.getByRole("switch", { name: `${app.label} Skill 分配` }),
      ).toBeVisible();
    }
    expect(
      screen
        .getAllByRole("switch")
        .map((node) => node.getAttribute("aria-label")),
    ).toEqual(SKILL_TARGETS.map((app) => `${app.label} Skill 分配`));

    const icons = Array.from(
      container.querySelectorAll<HTMLImageElement>(
        ".fy-feature-assignment-icon",
      ),
    );
    expect(icons).toHaveLength(7);
    for (const icon of icons) {
      expect(icon).toHaveAttribute("alt", "");
      expect(icon).toHaveAttribute("aria-hidden", "true");
      expect(icon.src).toMatch(/\/src\/v2\/shared\/assets\/(?:agents|apps)\//);
    }
    expect(screen.queryAllByRole("img")).toHaveLength(0);
  });

  it("preserves the switch accessible name and toggles the exact app ID", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <AssignmentPanel
        apps={createMcpAssignments()}
        disabled={false}
        labelSuffix="MCP 分配"
        onToggle={onToggle}
        targets={MCP_TARGETS}
      />,
    );

    await user.click(screen.getByRole("switch", { name: "OpenCode MCP 分配" }));

    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(onToggle).toHaveBeenCalledWith("opencode", true);
    expect(
      screen
        .getAllByRole("switch")
        .map((node) => node.getAttribute("aria-label")),
    ).toEqual(MCP_TARGETS.map((app) => `${app.label} MCP 分配`));
  });

  it("renders one seven-option install picker in catalog order", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const { container } = render(
      <AssignmentPanel
        mode="radio"
        ariaLabel="安装目标"
        onChange={onChange}
        targets={SKILL_TARGETS}
        value="claude"
      />,
    );

    expect(screen.getByRole("radiogroup", { name: "安装目标" })).toBeVisible();
    expect(
      screen.getAllByRole("radio").map((node) => node.textContent?.trim()),
    ).toEqual(SKILL_TARGETS.map((app) => app.label));
    expect(
      screen.getByRole("radio", { name: "Claude Code", checked: true }),
    ).toBeVisible();
    expect(
      container.querySelectorAll("img.fy-feature-assignment-icon"),
    ).toHaveLength(7);

    await user.click(screen.getByRole("radio", { name: "WorkBuddy" }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("workbuddy");
  });
});
