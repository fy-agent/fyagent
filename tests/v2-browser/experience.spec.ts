import path from "node:path";
import { expect, test } from "@playwright/test";

import { openV2Page, expectNoHorizontalOverflow } from "./support";
import { installRichTauriFeatureFixture } from "./support/features";

test("keeps saved-source switching in Auth and preserves the Agent return route", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openV2Page(
    page,
    "/models?target=codex&agentReturn=codex&agentSection=skills",
  );
  await expect(page.getByTestId("models-page")).toBeVisible();
  await expect(
    page.getByRole("region", { name: "切换已保存配置" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "管理 Codex 账号与来源" }).click();
  await expect(page).toHaveURL(
    /#\/auth\?consumer=codex&view=connections&agentReturn=codex&agentSection=skills/,
  );
  await expect(
    page.getByRole("region", { name: "切换已保存配置" }),
  ).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await page.getByRole("button", { name: "编辑 Codex 模型配置" }).click();
  await expect(page).toHaveURL(
    /#\/models\?target=codex&agentReturn=codex&agentSection=skills/,
  );
});

test("keeps shared hierarchy, rounded navigation and account dialog geometry consistent", async ({
  page,
}, info) => {
  await installRichTauriFeatureFixture(page);
  await openV2Page(page, "/agents");
  await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
  await expectNoHorizontalOverflow(page);
  const capture = async (name: string) => {
    await expect
      .poll(async () =>
        page.locator(".fy-side-navigation-track").evaluate((track) => {
          const selected = track.querySelector('[aria-current="page"]');
          const lens = track.querySelector(":scope > .fy-selection-lens");
          if (!selected || !lens) return Infinity;
          const target = selected.getBoundingClientRect();
          const overlay = lens.getBoundingClientRect();
          // SideNavigation deliberately insets its shared lens by 1px.
          return Math.max(
            Math.abs(target.top + 1 - overlay.top),
            Math.abs(target.height - 2 - overlay.height),
          );
        }),
      )
      .toBeLessThan(1.5);
    await page.screenshot({
      path: path.resolve(
        "node_modules/.cache/fyagent-ux",
        `${info.project.name}-${name}.jpg`,
      ),
      quality: 65,
      animations: "disabled",
    });
  };
  await capture("directory");
  const activeNav = page.locator(
    '.fy-side-navigation-item[aria-current="page"]',
  );
  await expect(activeNav).toBeVisible();
  expect(
    await activeNav.evaluate((node) =>
      parseFloat(getComputedStyle(node).borderTopLeftRadius),
    ),
  ).toBeGreaterThanOrEqual(21);
  await openV2Page(page, "/auth");
  await expect(
    page.getByRole("button", { name: "添加账号", exact: true }).first(),
  ).toBeVisible();
  await capture("accounts");
  expect(
    await page
      .getByRole("heading", { level: 1 })
      .evaluate((node) => getComputedStyle(node).fontSize),
  ).toBe("22px");
  await page
    .getByRole("button", { name: "添加账号", exact: true })
    .first()
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await capture("dialog");
  const metrics = await dialog.evaluate((node) => {
    const heading = node.querySelector("h2")!;
    const footer = node.querySelector("footer")!;
    const button = footer.querySelector("button")!;
    return {
      titleSize: getComputedStyle(heading).fontSize,
      titleWeight: getComputedStyle(heading).fontWeight,
      buttonSize: getComputedStyle(button).fontSize,
      headerHeight: node.querySelector("header")!.getBoundingClientRect()
        .height,
      footerHeight: footer.getBoundingClientRect().height,
      dialogHeight: node.getBoundingClientRect().height,
    };
  });
  await info.attach("dialog-metrics", {
    body: JSON.stringify(metrics),
    contentType: "application/json",
  });
  expect(metrics.titleSize).toBe("16px");
  expect(metrics.titleWeight).toBe("600");
  expect(metrics.buttonSize).toBe("13px");
  const viewport = page.viewportSize()!;
  const box = await dialog.boundingBox();
  expect(box!.width).toBeLessThanOrEqual(720);
  expect(box!.height).toBeLessThan(viewport.height);
  await expect(
    dialog.getByRole("button", { name: "取消", exact: true }),
  ).toBeInViewport();
  await expect(
    dialog.getByRole("button", { name: "下一步", exact: true }),
  ).toBeInViewport();
  await openV2Page(page, "/models?target=codex");
  await expect(page.getByTestId("models-page")).toBeVisible();
  await capture("models");
});
