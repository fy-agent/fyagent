import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
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
    await user.click(await screen.findByRole("button", { name: "更新" }));
    expect(await screen.findByText("Skill 更新完成失败")).toBeVisible();
    expect(
      screen.getByText(/部分目标更新未完成。已完成：.*未完成：Codex/),
    ).toBeVisible();
    expect(
      await screen.findByRole("heading", { name: "Updated Skill" }),
    ).toBeVisible();
    await waitFor(() => expect(getInstalled).toHaveBeenCalledTimes(2));
    expect(ports.skills.checkUpdates).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "更新" })).toBeEnabled();
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

  it("confirms the update-all list before updating, listing skill name and source repo", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    const skill: InstalledSkill = {
      id: "fixture/repo:review-skill",
      name: "Review Skill",
      directory: "review-skill",
      apps: createAssignments(["claude"]),
      installedAt: 1,
      updatedAt: 1,
      contentHash: "old",
      repoOwner: "fixture",
      repoName: "repo",
      repoBranch: "main",
    };
    ports.skills.getInstalled = vi.fn(async () => [{ ...skill }]);
    ports.skills.checkUpdates = vi.fn(async () => [
      { id: skill.id, name: skill.name, remoteHash: "new" },
      { id: "gone-skill", name: "Gone Skill", remoteHash: "new" },
    ]);
    ports.skills.update = vi.fn(async () => skill);
    const openExternal = vi.fn(async () => undefined);
    ports.settings.openExternal = openExternal;

    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );

    await screen.findByRole("heading", { name: "Review Skill" });
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    const updateAllButton = await screen.findByRole("button", {
      name: /更新全部/,
    });
    expect(updateAllButton).toBeVisible();

    await user.click(updateAllButton);
    expect(ports.skills.update).not.toHaveBeenCalled();

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Review Skill")).toBeVisible();
    expect(within(dialog).getByText("fixture/repo")).toBeVisible();
    expect(within(dialog).queryByText("Gone Skill")).not.toBeInTheDocument();

    const repoButton = within(dialog).getByRole("button", {
      name: "fixture/repo",
    });
    await user.click(repoButton);
    expect(openExternal).toHaveBeenCalledWith(
      "https://github.com/fixture/repo",
    );

    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(ports.skills.update).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: /更新全部/ }));
    const confirmDialog = await screen.findByRole("dialog");
    await user.click(
      within(confirmDialog).getByRole("button", { name: "确认" }),
    );
    await waitFor(() =>
      expect(ports.skills.update).toHaveBeenCalledWith(skill.id),
    );
    expect(ports.skills.update).toHaveBeenCalledTimes(1);
  });
});
