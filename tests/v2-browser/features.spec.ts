import { expect, test } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
} from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

for (const route of ["/skills", "/mcp"] as const) {
  test(`${route} remains responsive and keyboard reachable`, async ({
    page,
  }) => {
    const health = monitorPageHealth(page);
    await openV2Page(page, route);
    await expectNoHorizontalOverflow(page);
    await expect(page.getByTestId(`${route.slice(1)}-page`)).toBeVisible();
    await page.keyboard.press("Tab");
    await expect(page.locator(":focus")).toBeVisible();
    const viewportFits = await page
      .getByTestId("content-viewport")
      .evaluate((element) => element.scrollWidth <= element.clientWidth + 1);
    expect(viewportFits).toBe(true);
    await expectHealthyPage(page, health);
  });
}

test("MCP editor stays inside the viewport", async ({ page }) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/mcp");
  await page.getByRole("button", { name: "添加 MCP" }).first().click();
  const dialog = page.getByRole("dialog", { name: "添加 MCP" });
  await expect(dialog).toBeVisible();
  const box = await dialog.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.y).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(page.viewportSize()!.width);
  expect(box!.y + box!.height).toBeLessThanOrEqual(page.viewportSize()!.height);
  await expect(dialog.getByRole("button", { name: "保存" })).toBeVisible();
  await expectHealthyPage(page, health);
});

for (const feature of [
  {
    route: "/skills",
    pageTestId: "skills-page",
    list: "已安装 Skills 列表",
    detail: "Skill 详情",
    switchSuffix: "Skill 分配",
    targetCount: 7,
  },
  {
    route: "/mcp",
    pageTestId: "mcp-page",
    list: "MCP 列表",
    detail: "MCP 详情",
    switchSuffix: "MCP 分配",
    targetCount: 7,
  },
] as const) {
  test(`${feature.route} renders populated responsive master-detail-assignment data`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page);
    const health = monitorPageHealth(page);
    await openV2Page(page, feature.route);

    await expect(page.getByTestId(feature.pageTestId)).toBeVisible();
    await expect(
      page.getByRole("region", { name: feature.list }),
    ).toBeVisible();
    await expect(
      page.getByRole("region", { name: feature.detail }),
    ).toBeVisible();

    const switches = page.getByRole("switch", {
      name: new RegExp(`${feature.switchSuffix}$`),
    });
    await expect(switches).toHaveCount(feature.targetCount);
    const switchNames = await switches.evaluateAll((elements) =>
      elements.map((element) => element.getAttribute("aria-label")),
    );
    expect(switchNames).toEqual(
      [
        "QoderWork CN",
        "TRAE Work CN",
        "WorkBuddy",
        "Grok Build",
        "Codex",
        "Claude Code",
        "OpenCode",
      ].map((label) => `${label} ${feature.switchSuffix}`),
    );

    const paneCount = await page
      .locator(".fy-split-panes > .fy-split-pane")
      .count();
    expect(paneCount).toBe(page.viewportSize()!.width > 1180 ? 3 : 2);
    if ((page.viewportSize()?.width ?? 0) > 760) {
      await expect(
        page.getByRole("separator", { name: "调整列表与详情的宽度" }),
      ).toBeVisible();
    }
    const assignmentOverflow = await page
      .locator(".fy-split-pane .fy-feature-assignment")
      .evaluateAll((rows) =>
        rows.flatMap((row) => {
          const pane = row.closest(".fy-split-pane");
          if (!(pane instanceof HTMLElement)) {
            return ["assignment row is missing a split pane"];
          }
          const rowBox = row.getBoundingClientRect();
          const paneBox = pane.getBoundingClientRect();
          return rowBox.left < paneBox.left - 1 ||
            rowBox.right > paneBox.right + 1
            ? [`${row.textContent?.replace(/\s+/g, " ").trim()} overflows pane`]
            : [];
        }),
      );
    expect(assignmentOverflow).toEqual([]);
    await expectNoHorizontalOverflow(page);
    await expectHealthyPage(page, health);
  });
}

test("MCP fixture secrets stay out of ordinary populated UI", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/mcp");

  await expect(page.getByText("Fixture Context Server").first()).toBeVisible();
  await expect(page.locator("body")).not.toContainText(
    "synthetic-secret-never-render",
  );
  await expect(page.locator("body")).not.toContainText(
    "synthetic-header-never-render",
  );
  await expectHealthyPage(page, health);
});

test("Skill assignment invokes the command and refreshes authoritative data", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/skills");

  const codexSwitch = page.getByRole("switch", {
    name: "Codex Skill 分配",
  });
  await expect(codexSwitch).not.toBeChecked();
  await codexSwitch.click();
  await expect(codexSwitch).toBeChecked();

  await expect
    .poll(async () => {
      const calls = await featureFixtureCalls(page);
      return calls.filter((call) => call.command === "get_installed_skills")
        .length;
    })
    .toBeGreaterThanOrEqual(2);
  const calls = await featureFixtureCalls(page);
  expect(calls).toContainEqual({
    command: "toggle_skill_app",
    payload: {
      id: "fixture-review",
      app: "codex",
      enabled: true,
    },
  });
  await expectHealthyPage(page, health);
});
