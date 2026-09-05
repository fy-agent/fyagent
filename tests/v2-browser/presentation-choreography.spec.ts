import { expect, test, type Page } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import { expectHealthyPage, monitorPageHealth, openV2Page } from "./support";

// Inspect actual browser keyframes, not a production-only deterministic branch.
// MutationObserver sees the completed layout-effect setup before the next paint.
async function arm(page: Page, phase: "open" | "exit", time: number) {
  await page.evaluate(
    ({ phase, time }) => {
      document.documentElement.removeAttribute("data-presentation-paused");
      const observer = new MutationObserver(() => {
        const dialog = document.querySelector(
          `.fy-control-dialog[data-motion-phase="${phase}"]`,
        );
        const plane = dialog?.querySelector(".fy-dialog-material");
        if (!plane?.getAnimations().length) return;
        const overlay = document.querySelector(".fy-control-dialog-overlay");
        const animations = [
          ...dialog!.getAnimations({ subtree: true }),
          ...(overlay?.getAnimations() ?? []),
        ];
        for (const animation of animations) {
          animation.pause();
          animation.currentTime = time;
        }
        observer.disconnect();
        document.documentElement.setAttribute(
          "data-presentation-paused",
          "true",
        );
      });
      observer.observe(document.body, {
        subtree: true,
        childList: true,
        attributes: true,
        attributeFilter: ["data-motion-phase", "data-motion-origin"],
      });
    },
    { phase, time },
  );
}

async function paused(page: Page) {
  await expect(page.locator("html")).toHaveAttribute(
    "data-presentation-paused",
    "true",
  );
}

async function seek(page: Page, time?: number) {
  await page.evaluate((time) => {
    for (const root of document.querySelectorAll(
      ".fy-control-dialog, .fy-control-dialog-overlay",
    )) {
      for (const animation of root.getAnimations({ subtree: true })) {
        if (time === undefined) animation.play();
        else {
          animation.pause();
          animation.currentTime = time;
        }
      }
    }
  }, time);
}

async function sample(page: Page) {
  return page.locator(".fy-control-dialog").evaluate((dialog) => {
    const plane = dialog.querySelector(".fy-dialog-material")!;
    const foreground = dialog.querySelector(".fy-dialog-foreground")!;
    const rect = plane.getBoundingClientRect();
    const destination = dialog.getBoundingClientRect();
    const opacity = (element: Element) =>
      Number(getComputedStyle(element).opacity);
    return {
      rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      destination: {
        x: destination.x,
        y: destination.y,
        width: destination.width,
        height: destination.height,
      },
      planeOpacity: opacity(plane),
      contentOpacity: opacity(foreground),
      sourceOpacity: opacity(
        dialog.querySelector(".fy-dialog-source-material")!,
      ),
      targetOpacity: opacity(
        dialog.querySelector(".fy-dialog-target-material")!,
      ),
      overlayOpacity: opacity(
        document.querySelector(".fy-control-dialog-overlay")!,
      ),
      foregroundTransform: getComputedStyle(foreground).transform,
      radius: getComputedStyle(plane).borderTopLeftRadius,
    };
  });
}

test("the actual press remains visible before the modal backdrop takes over", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openV2Page(page, "/auth");
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  const original = await trigger.boundingBox();
  await trigger.hover();
  await page.mouse.down();
  await expect
    .poll(async () => (await trigger.boundingBox())!.width / original!.width)
    .toBeLessThan(0.99);
  await arm(page, "open", 40);
  await page.mouse.up();
  await paused(page);
  // The click already opened the real modal. Only its visual takeover waits
  // for the first press phase; no arbitrary timer delays the business action.
  await expect(page.getByRole("dialog")).toHaveCount(1);
  const early = await sample(page);
  expect(early.overlayOpacity).toBe(0);
  expect(early.planeOpacity).toBe(0);
  expect(early.contentOpacity).toBe(0);
  await seek(page);
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-motion-settled",
    "true",
  );
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(trigger).toBeFocused();
});

test("source, shell and content overlap at the reviewed forward and reverse keyframes", async ({
  page,
}, info) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/auth");
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  const source = await trigger.boundingBox();
  const radius = await trigger.evaluate(
    (node) => getComputedStyle(node).borderTopLeftRadius,
  );
  await arm(page, "open", 0);
  await trigger.evaluate((node) => (node as HTMLButtonElement).click());
  await paused(page);
  const opening = await sample(page);
  for (const key of ["x", "y", "width", "height"] as const)
    expect(Math.abs(opening.rect[key] - source![key])).toBeLessThanOrEqual(1);
  expect(opening.radius).toBe(radius);
  expect(opening.contentOpacity).toBe(0);
  expect(opening.overlayOpacity).toBe(0);
  await seek(page, 80);
  expect((await sample(page)).contentOpacity).toBe(0);
  await seek(page, 252);
  const handoff = await sample(page);
  expect(handoff.rect.width).toBeGreaterThan(handoff.destination.width * 0.85);
  expect(handoff.contentOpacity).toBe(0);
  await seek(page, 336);
  const overlap = await sample(page);
  expect(overlap.contentOpacity).toBeGreaterThan(0);
  expect(overlap.contentOpacity).toBeLessThan(1);
  expect(overlap.foregroundTransform).toBe("none");
  await info.attach("forward-handoff.png", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
  await seek(page);
  const dialog = page.getByRole("dialog", { name: "添加官方账号" });
  await expect(dialog).toHaveAttribute("data-motion-settled", "true");
  await arm(page, "exit", 0);
  await dialog
    .getByRole("button", { name: "取消", exact: true })
    .evaluate((node) => (node as HTMLButtonElement).click());
  await paused(page);
  await expect(dialog.locator("footer")).toHaveCount(0);
  await expect(dialog.locator(".fy-dialog-foreground")).toHaveAttribute(
    "inert",
    "",
  );
  await expect(dialog.locator(".fy-dialog-foreground")).toHaveAttribute(
    "aria-hidden",
    "true",
  );
  await expect(dialog.locator(".fy-control-dialog-body")).toHaveCount(1);
  await seek(page, 40);
  const departure = await sample(page);
  expect(departure.contentOpacity).toBeGreaterThan(0);
  expect(departure.contentOpacity).toBeLessThan(1);
  await seek(page, 270);
  const returning = await sample(page);
  expect(returning.contentOpacity).toBe(0);
  expect(returning.sourceOpacity).toBeGreaterThan(0.75);
  expect(returning.targetOpacity).toBeLessThan(0.1);
  await info.attach("return-handoff.png", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
  await seek(page);
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await info.attach("presentation-keyframes", {
    body: JSON.stringify({ opening, handoff, overlap, departure, returning }),
    contentType: "application/json",
  });
  await expectHealthyPage(page, health);
});

test("an interrupted close and reopen start from current geometry, not a full-size reset", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/auth");
  const trigger = page
    .getByRole("button", { name: "添加账号", exact: true })
    .first();
  await trigger.evaluate((node) =>
    node.setAttribute("data-test-origin", "true"),
  );
  await arm(page, "open", 200);
  await trigger.evaluate((node) => (node as HTMLButtonElement).click());
  await paused(page);
  const middle = await sample(page);
  await arm(page, "exit", 0);
  await page.keyboard.press("Escape");
  await paused(page);
  const reversed = await sample(page);
  for (const key of ["x", "y", "width", "height"] as const)
    expect(Math.abs(reversed.rect[key] - middle.rect[key])).toBeLessThanOrEqual(
      1,
    );
  await arm(page, "open", 0);
  await page
    .locator('[data-test-origin="true"]')
    .evaluate((node) => (node as HTMLButtonElement).click());
  await paused(page);
  const reopened = await sample(page);
  for (const key of ["x", "y", "width", "height"] as const)
    expect(
      Math.abs(reopened.rect[key] - reversed.rect[key]),
    ).toBeLessThanOrEqual(1);
  expect(reopened.contentOpacity).toBe(reversed.contentOpacity);
  await seek(page);
  await expect(page.getByRole("dialog")).toHaveAttribute(
    "data-motion-settled",
    "true",
  );
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expectHealthyPage(page, health);
});
