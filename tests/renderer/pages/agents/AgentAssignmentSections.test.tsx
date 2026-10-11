import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AgentSkillsSection } from "@/pages/agents/AgentAssignmentSections";
import { PRODUCT_DIRECTORY } from "@/shared/features/directory";
import { FeatureProvider } from "@/shared/features/provider";
import type { InstalledSkill } from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const codexEntry = PRODUCT_DIRECTORY.find((entry) => entry.agentId === "codex")!;

function makeSkill(enabled = false): InstalledSkill {
  return {
    id: "skill-test",
    name: "Test Skill",
    description: "A test skill for assignment",
    directory: "test-skill",
    apps: {
      qoderwork: false,
      "trae-work": false,
      workbuddy: false,
      grokbuild: false,
      codex: enabled,
      claude: false,
      opencode: false,
    },
    installedAt: 1000,
    updatedAt: 1000,
  };
}

describe("AgentSkillsSection error reporting and recovery state", () => {
  it("displays backend rejection reason when mutation is rejected", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = vi.fn(async () => [makeSkill(false)]);
    ports.skills.toggleApp = vi.fn(async () => {
      throw new Error("backend mutation rejected");
    });

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <AgentSkillsSection entry={codexEntry} onOpenManagement={vi.fn()} />
        </FeatureProvider>
      </QueryClientProvider>,
    );

    const switchBtn = await screen.findByRole("switch", {
      name: "在 Codex 中使用 Test Skill",
    });
    await user.click(switchBtn);

    await waitFor(
      () => {
        expect(
          screen.queryByText("正在保存 Skill 设置…"),
        ).not.toBeInTheDocument();
        const notices = screen.getAllByRole("status");
        const feedbackNotice = notices.find((n) =>
          n.textContent?.includes("Skill 设置"),
        );
        expect(feedbackNotice).toBeDefined();
        expect(feedbackNotice!.textContent).toContain(
          "无法确认 Skill 设置是否已更新。请刷新后重试。",
        );
        expect(feedbackNotice!.textContent).toContain("后端拒绝");
        expect(feedbackNotice!.textContent).toContain("backend mutation rejected");
      },
      { timeout: 3000 },
    );
  });

  it("displays native plain text rejection reason when mutation fails", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    ports.skills.getInstalled = vi.fn(async () => [makeSkill(false)]);
    ports.skills.toggleApp = vi.fn(async () => {
      throw "权限不足";
    });

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <AgentSkillsSection entry={codexEntry} onOpenManagement={vi.fn()} />
        </FeatureProvider>
      </QueryClientProvider>,
    );

    const switchBtn = await screen.findByRole("switch", {
      name: "在 Codex 中使用 Test Skill",
    });
    await user.click(switchBtn);

    await waitFor(
      () => {
        expect(
          screen.queryByText("正在保存 Skill 设置…"),
        ).not.toBeInTheDocument();
        const notices = screen.getAllByRole("status");
        const feedbackNotice = notices.find((n) =>
          n.textContent?.includes("Skill 设置"),
        );
        expect(feedbackNotice).toBeDefined();
        expect(feedbackNotice!.textContent).toContain(
          "无法确认 Skill 设置是否已更新。请刷新后重试。",
        );
        expect(feedbackNotice!.textContent).toContain("后端拒绝");
        expect(feedbackNotice!.textContent).toContain("权限不足");
      },
      { timeout: 3000 },
    );
  });

  it("displays read failure reason when reread fails", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    let callCount = 0;
    ports.skills.getInstalled = vi.fn(async () => {
      callCount++;
      if (callCount > 1) {
        throw new Error("readback network timeout");
      }
      return [makeSkill(false)];
    });
    ports.skills.toggleApp = vi.fn(async () => true);

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <AgentSkillsSection entry={codexEntry} onOpenManagement={vi.fn()} />
        </FeatureProvider>
      </QueryClientProvider>,
    );

    const switchBtn = await screen.findByRole("switch", {
      name: "在 Codex 中使用 Test Skill",
    });
    await user.click(switchBtn);

    await waitFor(
      () => {
        expect(
          screen.queryByText("正在保存 Skill 设置…"),
        ).not.toBeInTheDocument();
        const notices = screen.getAllByRole("status");
        const feedbackNotice = notices.find((n) =>
          n.textContent?.includes("Skill 设置"),
        );
        expect(feedbackNotice).toBeDefined();
        expect(feedbackNotice!.textContent).toContain(
          "无法确认 Skill 设置是否已更新。请刷新后重试。",
        );
        expect(feedbackNotice!.textContent).toContain("读取失败");
        expect(feedbackNotice!.textContent).toContain("readback network timeout");
      },
      { timeout: 3000 },
    );
  });

  it("displays mismatch reason and consumes recovery readback result", async () => {
    const user = userEvent.setup();
    const ports = createBrowserFeaturePorts();
    // Mutation succeeds, but readback returns false (mismatch).
    // The second (recovery) reread returns false as well.
    ports.skills.getInstalled = vi.fn(async () => [makeSkill(false)]);
    ports.skills.toggleApp = vi.fn(async () => true);

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <FeatureProvider ports={ports}>
          <AgentSkillsSection entry={codexEntry} onOpenManagement={vi.fn()} />
        </FeatureProvider>
      </QueryClientProvider>,
    );

    const switchBtn = await screen.findByRole("switch", {
      name: "在 Codex 中使用 Test Skill",
    });
    await user.click(switchBtn);

    await waitFor(
      () => {
        expect(
          screen.queryByText("正在保存 Skill 设置…"),
        ).not.toBeInTheDocument();
        const notices = screen.getAllByRole("status");
        const feedbackNotice = notices.find((n) =>
          n.textContent?.includes("Skill 设置"),
        );
        expect(feedbackNotice).toBeDefined();
        expect(feedbackNotice!.textContent).toContain(
          "无法确认 Skill 设置是否已更新。请刷新后重试。",
        );
        expect(feedbackNotice!.textContent).toContain("状态不一致");
        expect(feedbackNotice!.textContent).toContain("当前实际状态：已停用");
      },
      { timeout: 3000 },
    );
  });
});
