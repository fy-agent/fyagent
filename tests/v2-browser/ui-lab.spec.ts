import { expect, test } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
  requiredBox,
  tabToTestId,
} from "./support";

test("exercises UI Lab overlays, focus treatment, long labels, and glass fallback", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/__dev/ui-lab");

  const glassButton = page.getByTestId("ui-lab-glass-button");
  await expect(glassButton).toBeVisible();
  await expect(glassButton).toHaveAccessibleName("玻璃按钮");

  const glassTreatment = await glassButton.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      backdropFilter:
        style.backdropFilter ||
        style.getPropertyValue("-webkit-backdrop-filter"),
      backgroundColor: style.backgroundColor,
    };
  });
  expect(
    glassTreatment.backdropFilter !== "none" ||
      !["transparent", "rgba(0, 0, 0, 0)"].includes(
        glassTreatment.backgroundColor,
      ),
    "Glass controls need a visible non-transparent fallback surface",
  ).toBe(true);

  const tooltipTrigger = page.getByTestId("ui-lab-tooltip-trigger");
  await tooltipTrigger.hover();
  await expect(page.getByTestId("ui-lab-tooltip-content")).toBeVisible();
  await page.keyboard.press("Escape");

  const popoverTrigger = page.getByTestId("ui-lab-popover-trigger");
  await popoverTrigger.click();
  const popoverContent = page.getByTestId("ui-lab-popover-content");
  const popoverBox = await requiredBox(popoverContent, "UI Lab popover");
  const viewportSize = page.viewportSize();
  expect(viewportSize).not.toBeNull();
  expect(popoverBox.x).toBeGreaterThanOrEqual(-1);
  expect(popoverBox.y).toBeGreaterThanOrEqual(-1);
  expect(popoverBox.x + popoverBox.width).toBeLessThanOrEqual(
    viewportSize!.width + 1,
  );
  expect(popoverBox.y + popoverBox.height).toBeLessThanOrEqual(
    viewportSize!.height + 1,
  );
  expect(
    await popoverContent.evaluate((element) => {
      const contentViewport = document.querySelector(
        '[data-testid="content-viewport"]',
      );
      return contentViewport ? !contentViewport.contains(element) : false;
    }),
    "Popover content must be portaled outside the clipping content surface",
  ).toBe(true);
  await page.keyboard.press("Escape");

  await tabToTestId(page, "ui-lab-focus-target");
  const focusTarget = page.getByTestId("ui-lab-focus-target");
  await expect(focusTarget).toBeFocused();
  expect(
    await focusTarget.evaluate((element) => element.matches(":focus-visible")),
  ).toBe(true);
  const focusTreatment = await focusTarget.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      outlineStyle: style.outlineStyle,
      outlineWidth: Number.parseFloat(style.outlineWidth),
      boxShadow: style.boxShadow,
    };
  });
  expect(
    (focusTreatment.outlineStyle !== "none" &&
      focusTreatment.outlineWidth > 0) ||
      focusTreatment.boxShadow !== "none",
    "Keyboard focus must have a visible outline or ring",
  ).toBe(true);

  const longLabels = page.getByTestId("ui-lab-long-labels");
  await expect(longLabels).toBeVisible();
  const longLabelText = (await longLabels.textContent()) ?? "";
  expect(longLabelText).toMatch(/[A-Za-z]/);
  expect(longLabelText).toMatch(/[\u3040-\u30ff\u31f0-\u31ff]/);
  expect(longLabelText).toMatch(/[\u4e00-\u9fff]/);

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});
