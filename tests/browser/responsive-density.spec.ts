import { expect, test, type Page } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

const targets = [
  "QoderWork CN",
  "TRAE Work CN",
  "WorkBuddy",
  "Grok Build",
  "Codex",
  "Claude Code",
  "OpenCode",
];

async function resize(page: Page, width: number, height = 700) {
  await page.setViewportSize({ width, height });
  // Wait for native layout/observer delivery, not an arbitrary animation delay.
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
  });
}

for (const route of ["skills", "mcp"] as const) {
  for (const theme of ["light", "dark"] as const) {
    test(`${route} ${theme} assignment rows remain content-sized after narrow/wide re-entry`, async ({
      page,
    }, info) => {
      await page.addInitScript(
        (theme) => localStorage.setItem("fyagent-theme", theme),
        theme,
      );
      await installRichTauriFeatureFixture(page);
      const health = monitorPageHealth(page);
      await resize(page, 1564, 991);
      await openRendererPage(page, `/${route}`);
      for (const width of [1564, 1232, 1180, 900, 1181, 1564]) {
        await resize(page, width);
        const assignment = page
          .locator(".fy-feature-assignments:visible")
          .first();
        await expect(
          assignment.locator(".fy-feature-assignment-label"),
        ).toHaveText(targets);
        const rows = await assignment
          .locator(".fy-feature-assignment")
          .evaluateAll((nodes) =>
            nodes.map((node) => {
              const rect = node.getBoundingClientRect();
              const children = [...node.children].map((child) =>
                child.getBoundingClientRect(),
              );
              return {
                height: rect.height,
                content: Math.max(...children.map((box) => box.height)),
              };
            }),
          );
        for (const row of rows) {
          expect(
            row.height,
            `${route}/${theme}/${width}: intrinsic row height`,
          ).toBeLessThanOrEqual(Math.max(31, row.content) + 12);
        }
      }
      await resize(page, 1564, 991);
      await page.screenshot({
        path: `node_modules/.cache/fyagent-round8/${info.project.name}-${route}-${theme}.png`,
      });
      await expectNoHorizontalOverflow(page);
      await expectHealthyPage(page, health);
    });
  }

  test(`${route} gives extra width to detail and keeps bulk rows uniformly aligned`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page);
    const health = monitorPageHealth(page);
    await resize(page, 1564, 991);
    await openRendererPage(page, `/${route}`);
    const detail = page.locator(".fy-feature-detail-scroll");
    const rail = page.locator(".fy-feature-assign-scroll");
    await expect(detail).toBeVisible();
    expect((await rail.boundingBox())!.width).toBeLessThanOrEqual(361);
    expect((await detail.boundingBox())!.width).toBeGreaterThan(550);
    const wide = (await detail.boundingBox())!.width;
    await resize(page, 1232);
    const narrow = (await detail.boundingBox())!.width;
    expect(wide - narrow).toBeGreaterThan(200);
    for (const width of [1232, 1181, 1564]) {
      await resize(page, width);
      if (width === 1564) {
        expect((await rail.boundingBox())!.width).toBeLessThanOrEqual(361);
        expect((await detail.boundingBox())!.width).toBeGreaterThan(550);
      }
      const rows = await rail
        .locator(".fy-feature-assignment:not(label)")
        .evaluateAll((nodes) =>
          nodes.map((node) => {
            const label = node.firstElementChild!.getBoundingClientRect();
            const buttons = [...node.querySelectorAll("button")].map((button) =>
              button.getBoundingClientRect(),
            );
            return {
              stacked: buttons[0].top >= label.bottom - 1,
              tops: buttons.map((box) => box.top),
              right: Math.max(...buttons.map((box) => box.right)),
              rowRight: node.getBoundingClientRect().right,
            };
          }),
        );
      expect(rows).toHaveLength(7);
      expect(new Set(rows.map((row) => row.stacked)).size).toBe(1);
      for (const row of rows) {
        expect(Math.abs(row.tops[0] - row.tops[1])).toBeLessThan(1);
        expect(row.right).toBeLessThanOrEqual(row.rowRight + 1);
      }
      const cards = await page
        .locator(".fy-feature-info-grid")
        .evaluate((grid) => ({
          width: grid.getBoundingClientRect().width,
          children: [...grid.children].map(
            (node) => node.getBoundingClientRect().width,
          ),
        }));
      for (const card of cards.children)
        expect(card).toBeGreaterThanOrEqual(Math.min(cards.width, 256) - 1);
    }
    await expectNoHorizontalOverflow(page);
    await expectHealthyPage(page, health);
  });

  test(`${route} rail drag, keyboard and reset keep detail flexible without losing selection`, async ({
    page,
  }, info) => {
    await installRichTauriFeatureFixture(page);
    const health = monitorPageHealth(page);
    await resize(page, 1564, 991);
    await openRendererPage(page, `/${route}`);
    // Do not let macOS overlay scrollbars hide a duplicated gutter.
    await page.addStyleTag({
      content: "::-webkit-scrollbar { width: 15px; height: 15px; }",
    });
    const detail = page.locator(".fy-feature-detail-scroll");
    const detailNode = await detail.elementHandle();
    const rail = page.locator(".fy-feature-assign-scroll");
    const handle = page.getByRole("separator", {
      name: "调整详情与分配的宽度",
    });
    const drag = async (delta: number) => {
      const box = (await handle.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + 50);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + delta, box.y + 50, {
        steps: 6,
      });
      await page.mouse.up();
    };
    await drag(-500);
    await expect
      .poll(async () => (await rail.boundingBox())!.width)
      .toBeGreaterThan(350);
    expect((await rail.boundingBox())!.width).toBeLessThanOrEqual(361);
    await drag(500);
    await expect
      .poll(async () => (await rail.boundingBox())!.width)
      .toBeLessThan(230);
    expect((await rail.boundingBox())!.width).toBeGreaterThanOrEqual(219);
    await handle.focus();
    await page.keyboard.press("ArrowLeft");
    await expect
      .poll(async () => (await rail.boundingBox())!.width)
      .toBeGreaterThan(230);
    await handle.dblclick();
    await expect
      .poll(async () => Math.abs((await rail.boundingBox())!.width - 280))
      .toBeLessThan(2);
    const railWidth = (await rail.boundingBox())!.width;
    await resize(page, 1440, 900);
    expect(
      Math.abs((await rail.boundingBox())!.width - railWidth),
    ).toBeLessThan(2);
    for (const width of [900, 1181, 1564]) await resize(page, width, 991);
    expect(await detailNode!.evaluate((node) => node.isConnected)).toBe(true);
    expect((await detail.boundingBox())!.width).toBeGreaterThan(550);
    await page.screenshot({
      path: `node_modules/.cache/fyagent-round8/${info.project.name}-${route}.png`,
    });
    await expectHealthyPage(page, health);
  });
}

test("local Skill metadata, long content and enlarged text use bounded natural slots", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      const result = await invoke(command, payload);
      if (command !== "get_installed_skills") return result;
      return (result as Record<string, unknown>[]).map((skill, index) =>
        index
          ? skill
          : {
              ...skill,
              repoOwner: undefined,
              repoName: undefined,
              repoBranch: undefined,
              description: "这是用于容器宽度验收的说明。".repeat(10),
              path:
                "/fixture/private-location/" + "long-path-segment/".repeat(10),
            },
      );
    };
  });
  const health = monitorPageHealth(page);
  await resize(page, 1564, 991);
  await openRendererPage(page, "/skills");
  const sourceCard = page.getByRole("region", { name: "下载来源" });
  await expect(sourceCard).toContainText("本地导入");
  await expect(sourceCard).not.toContainText("/fixture/private-location/");
  await expect(
    sourceCard.getByRole("button", { name: "复制安装目录", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "node_modules/.cache/fyagent-round8/local-import-review.png",
  });
  await page.addStyleTag({
    content:
      ":root { --fy-font-control: 20px; } .fy-feature-assignment {font-size:20px} .fy-feature-definition, .fy-feature-info-card .fy-feature-definition, .fy-feature-info-card .fy-feature-definition dd {font-size:20px}",
  });
  for (const width of [1181, 900, 616, 1232, 1564]) {
    await resize(page, width, 900);
    const definitions = await page
      .locator(".fy-feature-definition:visible")
      .evaluateAll((nodes) =>
        nodes.map((node) => {
          const bounds = node.getBoundingClientRect();
          return {
            width: bounds.width,
            children: [...node.children].map((child) => {
              const box = child.getBoundingClientRect();
              return {
                width: box.width,
                right: box.right - bounds.right,
                left: bounds.left - box.left,
                tag: child.tagName,
              };
            }),
          };
        }),
      );
    for (const definition of definitions)
      for (const child of definition.children) {
        expect(child.right).toBeLessThanOrEqual(1);
        expect(child.left).toBeLessThanOrEqual(1);
        expect(child.width).toBeGreaterThan(0);
      }
    const bulkBounds = await page
      .locator(".fy-feature-bulk-row:visible")
      .evaluateAll((rows) =>
        rows.map((row) => ({
          right: row.getBoundingClientRect().right,
          buttons: [...row.querySelectorAll("button")].map(
            (button) => button.getBoundingClientRect().right,
          ),
        })),
      );
    for (const row of bulkBounds)
      for (const right of row.buttons)
        expect(right).toBeLessThanOrEqual(row.right + 1);
    await expectNoHorizontalOverflow(page);
  }
  await expectHealthyPage(page, health);
});

test("an MCP editor keeps its actual draft node across density breakpoints and route return", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await resize(page, 1564, 991);
  await openRendererPage(page, "/mcp");
  await page
    .getByRole("button", { name: "添加 MCP", exact: true })
    .first()
    .click();
  const dialog = page.getByRole("dialog", { name: "添加 MCP", exact: true });
  const name = dialog.getByRole("textbox", { name: "名称", exact: true });
  await name.fill("Keep this unsaved density draft");
  const node = await name.elementHandle();
  for (const width of [1232, 900, 616, 1181, 1564]) {
    await resize(page, width, 900);
    await expect(name).toHaveValue("Keep this unsaved density draft");
    expect(await node!.evaluate((element) => element.isConnected)).toBe(true);
  }
  // A kept-alive hidden route must clean its portal, without submitting the draft.
  await page
    .locator('.fy-side-navigation a[href="#/skills"]')
    .evaluate((node) => (node as HTMLAnchorElement).click());
  await expect(page.getByTestId("skills-page")).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("link", { name: "MCP 管理", exact: true }).click();
  await expect(name).toHaveValue("Keep this unsaved density draft");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      window.__FYAGENT_FEATURE_FIXTURE__.calls.filter(
        (call) => call.command === "upsert_mcp_server",
      ),
    ),
  ).toEqual([]);
  await expectHealthyPage(page, health);
});
