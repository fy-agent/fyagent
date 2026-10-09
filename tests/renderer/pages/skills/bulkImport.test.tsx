import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SkillsPage } from "@/pages/skills/Page";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import type {
  ImportSkillSelection,
  InstalledSkill,
  UnmanagedSkill,
} from "@/shared/features/types";

describe("local Skill import selection", () => {
  it("filters seventy observations, previews one, then writes and reads back only one", async () => {
    const ports = createBrowserFeaturePorts();
    const observations: UnmanagedSkill[] = Array.from(
      { length: 70 },
      (_, index) => ({
        directory: `directory-${index}`,
        name: index === 69 ? "unique-import" : `skill-${index}`,
        description: "",
        foundIn: ["claude"],
        path: `synthetic-observation-${index}`,
      }),
    );
    let installed: InstalledSkill[] = [];
    ports.skills.getInstalled = vi.fn(async () => installed);
    ports.skills.scanUnmanaged = vi.fn(async () => observations);
    ports.skills.importFromApps = vi.fn(async (imports) => {
      installed = imports.map((selection: ImportSkillSelection) => ({
        ...selection,
        id: selection.directory,
        name: "unique-import",
        installedAt: 1,
        updatedAt: 1,
      }));
      return installed;
    });
    const user = userEvent.setup();
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await user.click(await screen.findByRole("button", { name: "更多" }));
    await user.click(screen.getByRole("button", { name: "导入本地 Skill" }));
    const dialog = await screen.findByRole("dialog", {
      name: "导入本地 Skills",
    });
    expect(
      await within(dialog).findByRole("button", { name: "预览导入 · 0" }),
    ).toBeDisabled();
    await user.type(
      within(dialog).getByRole("searchbox", { name: "筛选本地 Skills" }),
      "unique-import",
    );
    await user.click(
      within(dialog).getByRole("button", { name: "选择筛选结果" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "预览导入 · 1" }),
    );
    expect(ports.skills.importFromApps).not.toHaveBeenCalled();
    await user.click(
      within(dialog).getByRole("button", { name: "确认导入 · 1" }),
    );
    await within(dialog).findByText("导入完成");
    expect(ports.skills.importFromApps).toHaveBeenCalledTimes(1);
    expect(
      vi
        .mocked(ports.skills.importFromApps)
        .mock.calls[0][0].map((entry) => entry.directory),
    ).toEqual(["directory-69"]);
    expect(ports.skills.getInstalled).toHaveBeenCalled();
  });
  it("locates a partially imported row after it disappears from unmanaged results", async () => {
    const ports = createBrowserFeaturePorts();
    const observation: UnmanagedSkill = {
      directory: "partial",
      name: "partial-skill",
      description: "",
      foundIn: ["claude"],
      path: "synthetic-partial",
    };
    let managed: InstalledSkill[] = [];
    ports.skills.getInstalled = vi.fn(async () => managed);
    ports.skills.scanUnmanaged = vi.fn(async () =>
      managed.length ? [] : [observation],
    );
    ports.skills.importFromApps = vi.fn(async (imports) => {
      managed = imports.map((selection: ImportSkillSelection) => ({
        ...selection,
        id: "partial-id",
        name: "partial-skill",
        installedAt: 0,
        updatedAt: 1,
        apps: { ...selection.apps, claude: false },
      }));
      return managed;
    });
    const user = userEvent.setup();
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await user.click(await screen.findByRole("button", { name: "更多" }));
    await user.click(screen.getByRole("button", { name: "导入本地 Skill" }));
    const dialog = await screen.findByRole("dialog", {
      name: "导入本地 Skills",
    });
    await within(dialog).findByRole("checkbox", {
      name: /选择 (partial|unknown)-skill/,
    });
    await user.click(
      within(dialog).getByRole("button", { name: "选择筛选结果" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "预览导入 · 1" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "确认导入 · 1" }),
    );
    expect(
      await within(dialog).findByText(
        "已入库，目标分配待确认或未完成；请在已安装页重新预览分配。",
      ),
    ).toBeInTheDocument();
    expect(
      within(dialog).queryByRole("button", { name: "选择未完成项重新预览" }),
    ).not.toBeInTheDocument();
    await user.click(
      within(dialog).getByRole("button", { name: "查看已安装 partial-skill" }),
    );
    expect(
      await screen.findByRole("heading", { name: "partial-skill" }),
    ).toBeInTheDocument();
    expect(ports.skills.importFromApps).toHaveBeenCalledTimes(1);
  });

  it("blocks blind reimport when the attempted import cannot be read back", async () => {
    const ports = createBrowserFeaturePorts();
    let attempted = false;
    const observation: UnmanagedSkill = {
      directory: "unknown",
      name: "unknown-skill",
      description: "",
      foundIn: ["claude"],
      path: "synthetic-unknown",
    };
    ports.skills.getInstalled = vi.fn(async () => {
      if (attempted) throw new Error("readback unavailable");
      return [];
    });
    ports.skills.scanUnmanaged = vi.fn(async () => [observation]);
    ports.skills.importFromApps = vi.fn(async () => {
      attempted = true;
      return [];
    });
    const user = userEvent.setup();
    render(
      <FeatureProvider ports={ports}>
        <SkillsPage />
      </FeatureProvider>,
    );
    await user.click(await screen.findByRole("button", { name: "更多" }));
    await user.click(screen.getByRole("button", { name: "导入本地 Skill" }));
    const dialog = await screen.findByRole("dialog", {
      name: "导入本地 Skills",
    });
    await within(dialog).findByRole("checkbox", {
      name: /选择 (partial|unknown)-skill/,
    });
    await user.click(
      within(dialog).getByRole("button", { name: "选择筛选结果" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "预览导入 · 1" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "确认导入 · 1" }),
    );
    expect(
      await within(dialog).findByText(
        "无法确认是否已入库；请刷新并查看已安装列表，避免重复导入。",
      ),
    ).toBeInTheDocument();
    expect(
      within(dialog).queryByRole("button", { name: "选择未完成项重新预览" }),
    ).not.toBeInTheDocument();
    await waitFor(() =>
      expect(
        within(dialog).getByRole("button", { name: "重新选择" }),
      ).toBeEnabled(),
    );
    await user.click(within(dialog).getByRole("button", { name: "重新选择" }));
    expect(
      await within(dialog).findByRole("button", { name: "预览导入 · 0" }),
    ).toBeDisabled();
    expect(ports.skills.importFromApps).toHaveBeenCalledTimes(1);
  });
});
