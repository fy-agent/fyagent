import { expect, test, type Locator, type Page } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  monitorPageHealth,
  openV2Page,
  expectNoHorizontalOverflow,
} from "./support";

async function paintedInk(page: Page, label: Locator) {
  const color = await label.evaluate((node) =>
    getComputedStyle(node)
      .color.match(/[\d.]+/g)!
      .map(Number),
  );
  const image = (await label.screenshot({ scale: "css" })).toString("base64");
  return page.evaluate(
    async ({ color, image }) => {
      const bitmap = new Image();
      bitmap.src = `data:image/png;base64,${image}`;
      await bitmap.decode();
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const ctx = canvas.getContext("2d")!;
      ctx.drawImage(bitmap, 0, 0);
      const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      let ink = 0;
      for (let i = 0; i < pixels.length; i += 4)
        if (
          Math.max(
            ...color
              .slice(0, 3)
              .map((channel, j) => Math.abs(channel - pixels[i + j])),
          ) < 35
        )
          ink++;
      return ink;
    },
    { color, image },
  );
}

async function promptsFixture(page: Page, populated: boolean) {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript((populated) => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      if (command === "get_prompts")
        return populated
          ? {
              fixture: {
                id: "fixture",
                name: "审查规范",
                content: "Initial fixture draft",
                description: "Deterministic layout fixture",
                enabled: false,
              },
            }
          : {};
      if (command === "get_current_prompt_file_content") return null;
      return invoke(command, payload);
    };
  }, populated);
}

test("selected shared tabs paint actual glyphs above the glass, including after switching", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  for (const route of ["/auth", "/skills", "/mcp"]) {
    await openV2Page(page, route);
    const scope = page.getByTestId(`${route.slice(1)}-page`);
    const tabs = scope.locator(".fy-feature-tab");
    await expect(tabs.first()).toBeVisible();
    for (let index = 0; index < Math.min(2, await tabs.count()); index++) {
      await tabs.nth(index).click();
      const selected = scope
        .locator('.fy-feature-tab[aria-selected="true"] .fy-feature-tab-label')
        .first();
      await expect(selected).toBeVisible();
      expect(await paintedInk(page, selected)).toBeGreaterThan(10);
    }
  }
  await expectHealthyPage(page, health);
});

for (const populated of [false, true])
  test(`prompts keep usable containers with ${populated ? "populated" : "empty"} data`, async ({
    page,
  }) => {
    await promptsFixture(page, populated);
    const health = monitorPageHealth(page);
    await openV2Page(page, "/prompts");
    const detail = page.locator(".fy-prompts-main-detail");
    await expect(detail).toBeVisible();
    await expect
      .poll(async () => (await detail.boundingBox())!.width)
      .toBeGreaterThan(280);
    const search = page.getByRole("searchbox", { name: "搜索提示词" });
    await expect(search).toBeVisible();
    expect((await search.boundingBox())!.width).toBeGreaterThan(180);
    if (populated) {
      const editor = page.getByRole("textbox", {
        name: "内容",
        exact: true,
      });
      await expect(editor).toHaveValue("Initial fixture draft");
      await editor.fill("Keep this unsaved draft");
      await page.setViewportSize({ width: 616, height: 800 });
      await expect(page.locator(".fy-prompts-master-detail")).toHaveAttribute(
        "data-stacked",
        "true",
      );
      await editor.scrollIntoViewIfNeeded();
      await expect(editor).toHaveValue("Keep this unsaved draft");
      expect((await editor.boundingBox())!.width).toBeGreaterThan(180);
      await page.setViewportSize({ width: 1440, height: 900 });
      await expect(editor).toHaveValue("Keep this unsaved draft");
    }
    await expectNoHorizontalOverflow(page);
    await expectHealthyPage(page, health);
  });

test("real separators clamp pointer and keyboard resizing, reset, and preserve the editor", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await promptsFixture(page, true);
  const health = monitorPageHealth(page);
  await openV2Page(page, "/prompts");
  const rail = page.locator(".fy-prompts-app-rail");
  const handle = page.getByRole("separator", { name: "调整目录与详情的宽度" });
  await expect(handle).toBeVisible();
  const drag = async (delta: number) => {
    const box = (await handle.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + 80);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + delta, box.y + 80, {
      steps: 6,
    });
    await page.mouse.up();
  };
  await drag(900);
  await expect
    .poll(async () => (await rail.boundingBox())!.width)
    .toBeGreaterThan(400);
  expect((await rail.boundingBox())!.width).toBeLessThanOrEqual(421);
  await drag(-900);
  await expect
    .poll(async () => (await rail.boundingBox())!.width)
    .toBeLessThan(240);
  expect((await rail.boundingBox())!.width).toBeGreaterThanOrEqual(219);
  await handle.focus();
  await page.keyboard.press("ArrowRight");
  await expect
    .poll(async () => (await rail.boundingBox())!.width)
    .toBeGreaterThan(240);
  await handle.dblclick();
  await expect
    .poll(async () => Math.abs((await rail.boundingBox())!.width - 268))
    .toBeLessThan(2);
  await expect(
    page.getByRole("textbox", { name: "内容", exact: true }),
  ).toHaveValue("Initial fixture draft");
  const activeHandle = (await handle.boundingBox())!;
  await page.mouse.move(
    activeHandle.x + activeHandle.width / 2,
    activeHandle.y + 60,
  );
  await page.mouse.down();
  await page.mouse.move(activeHandle.x + 45, activeHandle.y + 60);
  await page
    .locator('.fy-side-navigation a[href^="#/auth"]')
    .evaluate((node) => (node as HTMLAnchorElement).click());
  await expect(page.getByTestId("auth-page")).toBeVisible();
  await page.mouse.up();
  const accountSearch = page.getByPlaceholder("搜索账号");
  await accountSearch.fill("synthetic-layout-check");
  await expect(accountSearch).toHaveValue("synthetic-layout-check");
  expect(
    await page.evaluate(() => getComputedStyle(document.body).cursor),
  ).not.toMatch(/resize/);
  await expectHealthyPage(page, health);
});
