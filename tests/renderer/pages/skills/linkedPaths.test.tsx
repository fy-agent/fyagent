import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SkillsPage } from "@/pages/skills/Page";
import { FeatureProvider } from "@/shared/features/provider";
import {
  createAssignments,
  type InstalledSkill,
} from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

afterEach(cleanup);

function fixture(readOnly: boolean): InstalledSkill {
  return {
    id: "local:linked",
    name: "Observed Skill",
    directory: "linked",
    apps: createAssignments(["claude"]),
    installedAt: 0,
    updatedAt: 0,
    readOnly,
    readOnlyTargets: readOnly ? ["claude"] : [],
  };
}

describe("Skills read-only observation", () => {
  it("explains a detected linked resource and disables its mutation", async () => {
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = async () => [fixture(true)];
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await screen.findByRole("heading", { name: "Observed Skill" });
    expect(screen.getByText(/检测到链接目录：/)).toBeVisible();
    expect(screen.getByText(/只读目标：Claude/)).toBeVisible();
    expect(screen.getByRole("button", { name: /^卸载$/ })).toBeDisabled();
  });

  it("keeps ordinary assignment targets available beside a linked projection", async () => {
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = async () => [
      { ...fixture(false), readOnlyTargets: ["claude"] },
    ];
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await screen.findByRole("heading", { name: "Observed Skill" });
    expect(
      screen.getByRole("switch", { name: "Claude Code Skill 分配" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("switch", { name: "Codex Skill 分配" }),
    ).toBeEnabled();
  });

  it("does not mark an ordinary observed resource read-only", async () => {
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = async () => [fixture(false)];
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await screen.findByRole("heading", { name: "Observed Skill" });
    expect(screen.queryByText(/检测到链接目录：/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^卸载$/ })).toBeEnabled();
  });
});
