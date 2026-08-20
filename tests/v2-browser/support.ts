import { expect, type Locator, type Page } from "@playwright/test";

export interface PageHealthMonitor {
  consoleErrors: string[];
  pageErrors: string[];
}

export interface ElementBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

const faviconRoutes = new WeakSet<Page>();

async function interceptKnownAssetRequests(page: Page): Promise<void> {
  if (faviconRoutes.has(page)) return;
  faviconRoutes.add(page);
  await page.route("**/favicon.ico", (route) =>
    route.fulfill({ status: 204, body: "" }),
  );
}

export function monitorPageHealth(page: Page): PageHealthMonitor {
  const monitor: PageHealthMonitor = {
    consoleErrors: [],
    pageErrors: [],
  };

  page.on("console", (message) => {
    if (message.type() === "error") {
      monitor.consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    monitor.pageErrors.push(error.stack ?? error.message);
  });

  return monitor;
}

export async function openV2Page(page: Page, route: string): Promise<void> {
  await interceptKnownAssetRequests(page);
  await page.goto(`/#${route}`);
  await expect(page).toHaveTitle("FyAgent");
  await expect(page.getByTestId("top-bar")).toBeVisible();
}

export async function expectHealthyPage(
  page: Page,
  monitor: PageHealthMonitor,
): Promise<void> {
  await expect(
    page.locator(
      "vite-error-overlay, nextjs-portal, #webpack-dev-server-client-overlay",
    ),
  ).toHaveCount(0);
  expect(
    monitor.pageErrors,
    `Unexpected page errors:\n${monitor.pageErrors.join("\n")}`,
  ).toEqual([]);
  expect(
    monitor.consoleErrors,
    `Unexpected console errors:\n${monitor.consoleErrors.join("\n")}`,
  ).toEqual([]);
}

export async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const metrics = await page.evaluate(() => ({
    bodyScrollWidth: document.body.scrollWidth,
    documentScrollWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
  }));

  expect(metrics.bodyScrollWidth).toBeLessThanOrEqual(
    metrics.viewportWidth + 1,
  );
  expect(metrics.documentScrollWidth).toBeLessThanOrEqual(
    metrics.viewportWidth + 1,
  );
}

export async function requiredBox(
  locator: Locator,
  label: string,
): Promise<ElementBox> {
  await expect(locator, `${label} must be visible`).toBeVisible();
  const box = await locator.boundingBox();
  expect(box, `${label} must have a rendered bounding box`).not.toBeNull();
  return box as ElementBox;
}

export function boxesOverlap(first: ElementBox, second: ElementBox): boolean {
  const horizontalIntersection =
    Math.min(first.x + first.width, second.x + second.width) -
    Math.max(first.x, second.x);
  const verticalIntersection =
    Math.min(first.y + first.height, second.y + second.height) -
    Math.max(first.y, second.y);

  return horizontalIntersection > 0.5 && verticalIntersection > 0.5;
}

export async function tabToTestId(
  page: Page,
  testId: string,
  maximumTabs = 40,
): Promise<void> {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });

  for (let index = 0; index < maximumTabs; index += 1) {
    await page.keyboard.press("Tab");
    const activeTestId = await page.evaluate(() =>
      document.activeElement?.getAttribute("data-testid"),
    );

    if (activeTestId === testId) {
      return;
    }
  }

  throw new Error(
    `Keyboard focus did not reach [data-testid="${testId}"] within ${maximumTabs} Tab presses.`,
  );
}
