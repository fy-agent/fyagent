import { expect, test, type Locator } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openV2Page,
  requiredBox,
  tabToTestId,
} from "./support";

interface SurfaceTreatment {
  backdropFilter: string;
  backgroundAlpha: number;
  backgroundColor: string;
  backgroundImage: string;
  borderColor: string;
  boxShadow: string;
}

async function readSurfaceTreatment(
  locator: Locator,
): Promise<SurfaceTreatment> {
  await expect(locator).toBeVisible();

  return locator.evaluate((element) => {
    const style = getComputedStyle(element);
    const backgroundColor = style.backgroundColor;
    const channels = backgroundColor.match(/[\d.]+/g)?.map(Number) ?? [];
    const backgroundAlpha =
      backgroundColor === "transparent"
        ? 0
        : channels.length >= 4
          ? channels[3]
          : 1;

    return {
      backdropFilter:
        style.backdropFilter ||
        style.getPropertyValue("-webkit-backdrop-filter"),
      backgroundAlpha,
      backgroundColor,
      backgroundImage: style.backgroundImage,
      borderColor: style.borderColor,
      boxShadow: style.boxShadow,
    };
  });
}

function expectGlassSurface(treatment: SurfaceTreatment, label: string): void {
  expect(
    treatment.backgroundAlpha,
    `${label} must retain a visible CSS tint fallback`,
  ).toBeGreaterThan(0);
  expect(
    treatment.backgroundAlpha,
    `${label} must remain translucent rather than opaque`,
  ).toBeLessThan(0.99);
  expect(
    treatment.backdropFilter !== "none" ||
      treatment.backgroundAlpha > 0 ||
      treatment.backgroundImage !== "none",
    `${label} must use backdrop treatment or a real CSS surface fallback`,
  ).toBe(true);
  expect(
    treatment.borderColor,
    `${label} must expose a visible glass edge`,
  ).not.toBe("rgba(0, 0, 0, 0)");
  expect(
    treatment.boxShadow,
    `${label} must expose highlight/depth through box shadow`,
  ).not.toBe("none");
}

function maximumTransitionSeconds(value: string): number {
  return Math.max(
    ...value.split(",").map((duration) => {
      const normalized = duration.trim();
      const amount = Number.parseFloat(normalized);
      return normalized.endsWith("ms") ? amount / 1_000 : amount;
    }),
  );
}

test("exercises UI Lab overlays, focus treatment, long labels, and glass fallback", async ({
  page,
}) => {
  const health = monitorPageHealth(page);
  await openV2Page(page, "/__dev/ui-lab");

  const navigation = page.getByRole("navigation", { name: "主导航" });
  await expect(navigation.locator('a[aria-current="page"]')).toHaveCount(0);

  const lenses = page.getByTestId("liquid-glass-lens");
  await expect(lenses).toHaveCount(1);
  expectGlassSurface(
    await readSurfaceTreatment(lenses),
    "UI Lab selected-lens specimen",
  );
  expectGlassSurface(
    await readSurfaceTreatment(navigation),
    "Primary navigation track",
  );

  const glassButton = page.getByTestId("ui-lab-glass-button");
  await expect(glassButton).toBeVisible();
  await expect(glassButton).toHaveAccessibleName("开始管理");
  expectGlassSurface(await readSurfaceTreatment(glassButton), "Glass control");

  const tooltipTrigger = page.getByTestId("ui-lab-tooltip-trigger");
  await tooltipTrigger.hover();
  await expect(page.getByTestId("ui-lab-tooltip-content")).toBeVisible();
  await page.keyboard.press("Escape");

  const popoverTrigger = page.getByTestId("ui-lab-popover-trigger");
  await popoverTrigger.click();
  const popoverContent = page.getByTestId("ui-lab-popover-content");
  expectGlassSurface(
    await readSurfaceTreatment(popoverContent),
    "Portaled popover",
  );
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
  expect(longLabelText.length).toBeGreaterThan(24);
  expect(longLabelText).toMatch(/[\u4e00-\u9fff]/);

  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(lenses).toHaveCount(1);
  await expect(lenses).toBeVisible();
  expectGlassSurface(
    await readSurfaceTreatment(lenses),
    "Reduced-motion selected-lens specimen",
  );
  await expect(page.getByRole("tab", { name: "已启用" })).toHaveAttribute(
    "data-state",
    "active",
  );
  const reducedTransitionDuration = await focusTarget.evaluate(
    (element) => getComputedStyle(element).transitionDuration,
  );
  expect(
    maximumTransitionSeconds(reducedTransitionDuration),
    "Reduced motion must remove perceptual control transitions without removing state",
  ).toBeLessThanOrEqual(0.001);

  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});
