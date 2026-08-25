import { expect, test, type Locator } from "@playwright/test";

import {
  boxesOverlap,
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
  requiredBox,
} from "./support";

const navigationContract = [
  { path: "/agents", label: "AI软件配置" },
  { path: "/models", label: "模型管理" },
  { path: "/skills", label: "Skills 管理" },
  { path: "/mcp", label: "MCP 管理" },
  { path: "/prompts", label: "提示词管理" },
  { path: "/memory", label: "记忆模块" },
] as const;

const visibleControlTestIds = ["search", "settings", "avatar"] as const;

const shellRegionTestIds = ["brand", "tool-cluster"] as const;

const windowControlNames = ["最小化", "最大化/还原", "关闭"] as const;

const primaryControlTestIds = [
  ...visibleControlTestIds,
  "#/agents",
  "configuration-management-toggle",
  "#/models",
  "#/skills",
  "#/mcp",
  "#/prompts",
  "#/memory",
] as const;

function routeLink(navigation: Locator, label: string): Locator {
  return navigation.getByRole("link", { name: label, exact: true });
}

function escapedRegularExpression(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

test("keeps the complete shell visible, separate, and overflow-free", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  await expectNoHorizontalOverflow(page);

  const topBar = page.getByTestId("top-bar");
  const topBarFits = await topBar.evaluate(
    (element) => element.scrollWidth <= element.clientWidth + 1,
  );
  expect(topBarFits, "TopBar must not overflow horizontally").toBe(true);
  expect(
    await topBar.evaluate((element) =>
      Array.from(
        element.querySelectorAll(
          '[data-testid="brand"], [data-testid="tool-cluster"]',
        ),
      ).map((region) => region.getAttribute("data-testid")),
    ),
    "TopBar must expose only Brand and Tools",
  ).toEqual([...shellRegionTestIds]);
  await expect(topBar.getByRole("navigation")).toHaveCount(0);

  await expect(page.locator("[data-tauri-drag-region]")).toHaveCount(0);
  await expect(page.getByTestId("titlebar-drag-region")).toHaveCount(0);
  await expect(page.getByTestId("window-controls")).toHaveCount(0);
  for (const name of windowControlNames) {
    await expect(page.getByRole("button", { name })).toHaveCount(0);
  }

  const regionBoxes = new Map<
    string,
    Awaited<ReturnType<typeof requiredBox>>
  >();
  for (const testId of shellRegionTestIds) {
    regionBoxes.set(
      testId,
      await requiredBox(page.getByTestId(testId), testId),
    );
  }

  for (
    let firstIndex = 0;
    firstIndex < shellRegionTestIds.length;
    firstIndex += 1
  ) {
    for (
      let secondIndex = firstIndex + 1;
      secondIndex < shellRegionTestIds.length;
      secondIndex += 1
    ) {
      const firstId = shellRegionTestIds[firstIndex];
      const secondId = shellRegionTestIds[secondIndex];
      const firstBox = regionBoxes.get(firstId);
      const secondBox = regionBoxes.get(secondId);

      expect(firstBox).toBeDefined();
      expect(secondBox).toBeDefined();
      expect(
        boxesOverlap(firstBox!, secondBox!),
        `${firstId} must not overlap ${secondId}`,
      ).toBe(false);
    }
  }

  const navigation = page.getByRole("navigation", { name: "主导航" });
  await expect(navigation).toHaveAttribute("data-testid", "side-navigation");
  await expect(
    navigation.getByText("AI软件配置", { exact: true }),
  ).toBeVisible();
  const configurationToggle = navigation.getByRole("button", {
    name: "配置管理",
  });
  await expect(configurationToggle).toBeVisible();
  await expect(configurationToggle).toHaveAttribute("aria-expanded", "true");
  await expect(navigation.getByText("记忆模块", { exact: true })).toBeVisible();
  await expect(
    navigation.locator(
      ".fy-side-navigation-group > .fy-side-navigation-item, .fy-side-navigation-group > .fy-side-navigation-toggle",
    ),
  ).toHaveCount(3);
  await expect(
    navigation.getByRole("link", { name: "Agent 目录" }),
  ).toHaveCount(0);
  await expect(
    navigation.getByRole("link", { name: "记忆", exact: true }),
  ).toHaveCount(0);
  const primaryControls: Locator[] = [];
  for (const { label } of navigationContract) {
    const link = routeLink(navigation, label);
    await expect(link).toBeVisible();
    primaryControls.push(link);
  }
  for (const testId of visibleControlTestIds) {
    const control = page.getByTestId(testId);
    await expect(control).toBeVisible();
    primaryControls.push(control);
  }
  primaryControls.push(configurationToggle);

  const viewportSize = page.viewportSize();
  expect(viewportSize).not.toBeNull();
  for (const [index, control] of primaryControls.entries()) {
    const box = await requiredBox(control, `primary control ${index + 1}`);
    expect(box.x).toBeGreaterThanOrEqual(-1);
    expect(box.y).toBeGreaterThanOrEqual(-1);
    expect(box.x + box.width).toBeLessThanOrEqual(viewportSize!.width + 1);
    expect(box.y + box.height).toBeLessThanOrEqual(viewportSize!.height + 1);
  }

  await expect(page.getByTestId("liquid-glass-lens")).toHaveCount(1);

  const contentViewport = page.getByTestId("content-viewport");
  const contentBox = await requiredBox(contentViewport, "content viewport");
  const navigationBox = await requiredBox(navigation, "side navigation");
  expect(contentBox.width).toBeGreaterThan(0);
  expect(contentBox.height).toBeGreaterThan(0);
  expect(
    boxesOverlap(navigationBox, contentBox),
    "Side navigation must not overlap the content viewport",
  ).toBe(false);
  expect(
    await contentViewport.evaluate((element) => element.textContent?.trim()),
  ).not.toBe("");

  await expectHealthyPage(page, health);
});

test("keeps hash, selected link, and aria-current aligned for every route", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  const navigation = page.getByRole("navigation", { name: "主导航" });
  for (const { path, label } of navigationContract) {
    const link = routeLink(navigation, label);
    await link.click();

    await expect(page).toHaveURL(
      new RegExp(`${escapedRegularExpression(`#${path}`)}$`),
    );
    await expect(link).toHaveAttribute("aria-current", "page");
    const selectedLinks = navigation.locator('a[aria-current="page"]');
    await expect(selectedLinks).toHaveCount(1);
    await expect(selectedLinks).toHaveText(label);
    const lenses = page.getByTestId("liquid-glass-lens");
    await expect(lenses).toHaveCount(1);
    await expect(link.getByTestId("liquid-glass-lens")).toHaveCount(1);
    await expect(link).toHaveClass(/fy-side-navigation-item-selected/);
    await expect(page.getByTestId("content-viewport")).not.toHaveText("");

    const selectedLens = navigation.getByTestId("selection-lens");
    await expect(selectedLens).toHaveCount(1);
    await expect
      .poll(
        () =>
          selectedLens.evaluate((element) => {
            const style = getComputedStyle(element);
            return (
              style.borderColor !== "rgba(0, 0, 0, 0)" &&
              style.boxShadow !== "none" &&
              (style.backgroundColor !== "rgba(0, 0, 0, 0)" ||
                style.backgroundImage !== "none")
            );
          }),
        {
          message:
            "Selected navigation must settle on a CSS border, surface, and shadow independent of the SVG filter",
        },
      )
      .toBe(true);

    const selectedTreatment = await link.evaluate((element) => {
      const after = getComputedStyle(element, "::after");
      return {
        afterContent: after.content,
      };
    });
    expect(
      ["none", "normal"],
      "Selected navigation must not use the retired underline pseudo-element",
    ).toContain(selectedTreatment.afterContent);
  }

  await expectHealthyPage(page, health);
});

test("reaches every primary control with the keyboard in document order", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  expect(
    await page
      .getByTestId("app-shell")
      .evaluate(
        (element) =>
          Array.from(
            element.querySelectorAll<HTMLElement>(
              'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
            ),
          ).filter((control) => control.tabIndex >= 0).length,
      ),
    "Renderer shell must contain the complete grouped keyboard path",
  ).toBe(primaryControlTestIds.length);

  const focusedControlIds: string[] = [];
  for (let index = 0; index < primaryControlTestIds.length; index += 1) {
    await page.keyboard.press("Tab");
    focusedControlIds.push(
      (await page.evaluate(() => {
        const activeElement = document.activeElement;
        return (
          activeElement?.getAttribute("data-testid") ??
          activeElement?.getAttribute("href")
        );
      })) ?? "",
    );
  }

  expect(focusedControlIds).toEqual([...primaryControlTestIds]);

  await expectHealthyPage(page, health);
});

test("supports expand, collapse, focus movement, and reduced motion", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const health = monitorPageHealth(page);
  await openV2Page(page, "/agents");

  const navigation = page.getByRole("navigation", { name: "主导航" });
  const configurationToggle = navigation.getByRole("button", {
    name: "配置管理",
  });
  const configurationItems = page.getByTestId("configuration-management-items");
  const caret = page.getByTestId("configuration-management-caret");

  await expect(configurationToggle).toHaveAttribute("aria-expanded", "true");
  await expect(configurationItems).toBeVisible();
  expect(
    await caret.evaluate(
      (element) => getComputedStyle(element).transitionDuration,
    ),
  ).toBe("0s");

  await configurationToggle.focus();
  await page.keyboard.press("Enter");
  await expect(configurationToggle).toHaveAttribute("aria-expanded", "false");
  await expect(configurationItems).toBeHidden();

  await page.keyboard.press("ArrowRight");
  await expect(configurationToggle).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("ArrowRight");
  const modelsLink = routeLink(navigation, "模型管理");
  await expect(modelsLink).toBeFocused();
  expect(
    await modelsLink.evaluate(
      (element) => getComputedStyle(element).outlineStyle,
    ),
  ).not.toBe("none");

  await page.keyboard.press("ArrowDown");
  await expect(routeLink(navigation, "Skills 管理")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(configurationToggle).toBeFocused();
  await expect(configurationToggle).toHaveAttribute("aria-expanded", "false");

  await expectHealthyPage(page, health);
});
