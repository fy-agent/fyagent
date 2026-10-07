import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SkillsPage } from "@/pages/skills/Page";
import { FeatureProvider } from "@/shared/features/provider";
import { skillUpdateErrorMessage } from "@/shared/features/skills";
import {
  createAssignments,
  type InstalledSkill,
} from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const partial = JSON.stringify({
  code: "UPDATE_INCOMPLETE",
  context: { applied: "claude", failed: "codex", conflicted: "" },
  suggestion: "retryFailedTargets",
});

afterEach(cleanup);

describe("Skills update failure recovery", () => {
  it("reads back changed metadata after partial failure, reports targets and retains retry", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    let skill: InstalledSkill = {
      id: "fixture/repo:review-skill",
      name: "Review Skill",
      directory: "review-skill",
      apps: createAssignments(["claude", "codex"]),
      installedAt: 1,
      updatedAt: 1,
      contentHash: "old",
      repoOwner: "fixture",
      repoName: "repo",
      repoBranch: "main",
    };
    const getInstalled = vi.fn(async () => [{ ...skill }]);
    ports.skills.getInstalled = getInstalled;
    ports.skills.checkUpdates = vi.fn(async () => [
      { id: skill.id, name: skill.name, remoteHash: "new" },
    ]);
    ports.skills.update = vi.fn(async () => {
      skill = {
        ...skill,
        name: "Updated Skill",
        contentHash: "new",
        updatedAt: 2,
      };
      throw partial;
    });
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await screen.findByRole("heading", { name: "Review Skill" });
    await user.click(screen.getByRole("button", { name: "检查更新" }));
    await user.click(
      await screen.findByRole("button", { name: "更新", exact: true }),
    );
    expect(await screen.findByText("Skill 更新完成失败")).toBeVisible();
    expect(
      screen.getByText(/部分目标更新未完成。已完成：.*未完成：Codex/),
    ).toBeVisible();
    expect(
      await screen.findByRole("heading", { name: "Updated Skill" }),
    ).toBeVisible();
    await waitFor(() => expect(getInstalled).toHaveBeenCalledTimes(2));
    expect(ports.skills.checkUpdates).toHaveBeenCalledTimes(2);
    expect(
      screen.getByRole("button", { name: "更新", exact: true }),
    ).toBeEnabled();
    expect(
      screen.queryByText("Skill 更新完成", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("keeps arbitrary native strings and unsafe target values out of the UI", () => {
    expect(skillUpdateErrorMessage("secret C:/private/token")).toBeUndefined();
    expect(
      skillUpdateErrorMessage(
        JSON.stringify({
          code: "UPDATE_INCOMPLETE",
          context: {
            applied: "claude",
            failed: "C:/private/token",
            conflicted: "",
          },
        }),
      ),
    ).toBeUndefined();
    expect(
      skillUpdateErrorMessage(
        JSON.stringify({ code: "UPDATE_BACKUP_FAILED", context: {} }),
      ),
    ).toContain("更新未执行");
    expect(
      skillUpdateErrorMessage(
        JSON.stringify({
          code: "UPDATE_INCOMPLETE",
          context: {
            applied: "claude,codex",
            failed: "claude",
            conflicted: "claude",
          },
        }),
      ),
    ).toContain("后续修改已保留");
  });
});
