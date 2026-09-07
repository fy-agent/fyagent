import { expect, test, type Locator, type Page } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";

async function installLongFixture(page: Page) {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      if (command === "get_prompts") {
        return Object.fromEntries(
          Array.from({ length: 40 }, (_, i) => [
            `scroll-${i}`,
            {
              id: `scroll-${i}`,
              name: `Scroll Prompt ${i}`,
              content: "Long prompt line\n".repeat(250),
              enabled: false,
            },
          ]),
        );
      }
      if (command === "get_current_prompt_file_content") return null;
      if (command === "read_workspace_file" || command === "get_hermes_memory")
        return "Long memory fixture line\n".repeat(250);
      if (command === "list_daily_memory_files") return [];
      if (command === "get_hermes_memory_limits")
        return {
          memory: 2200,
          user: 1400,
          memoryEnabled: true,
          userEnabled: true,
        };
      const result = await invoke(command, payload);
      if (command === "managed_auth_get_overview") {
        const overview = result as { accounts: Array<Record<string, unknown>> };
        return {
          ...overview,
          accounts: [
            ...overview.accounts,
            ...Array.from({ length: 38 }, (_, i) => ({
              ...overview.accounts[0],
              accountId: `ma1:${(i + 1).toString(16).padStart(32, "0")}`,
              login: `scroll-account-${i}@example.test`,
              isDefault: false,
              connectedConsumerCount: 0,
            })),
          ],
        };
      }
      if (command === "get_workbuddy_model_ids")
        return {
          ...(result as Record<string, unknown>),
          ids: Array.from({ length: 80 }, (_, i) => `fixture-model-${i}`),
        };
      if (command === "get_installed_skills") {
        const sample = (result as Array<Record<string, unknown>>)[0];
        return Array.from({ length: 40 }, (_, i) => ({
          ...sample,
          id: `scroll-${i}`,
          name: `Scroll Skill ${i}`,
          directory: `scroll-${i}`,
        }));
      }
      if (command === "get_mcp_servers") {
        const sample = Object.values(
          result as Record<string, Record<string, unknown>>,
        )[0];
        return Object.fromEntries(
          Array.from({ length: 40 }, (_, i) => [
            `scroll-${i}`,
            { ...sample, id: `scroll-${i}`, name: `Scroll MCP ${i}` },
          ]),
        );
      }
      if (command === "search_skillhub") {
        return {
          skills: Array.from({ length: 20 }, (_, i) => ({
            key: `market-${i}`,
            slug: `market-${i}`,
            name: `Market Skill ${i}`,
            description: "Long catalogue fixture. ".repeat(8),
            directory: `market-${i}`,
            repoOwner: "fixture",
            repoName: "skills",
            repoBranch: "main",
            homepageUrl: "https://example.test/skills",
          })),
          totalCount: 20,
          query: "",
          categories: [],
        };
      }
      return result;
    };
  });
}

async function wheelInside(page: Page, area: Locator, delta: number) {
  const box = await area.boundingBox();
  if (!box) throw new Error("Missing visible scroll region");
  await page.mouse.move(
    box.x + Math.min(box.width / 2, 100),
    Math.min(box.y + 80, page.viewportSize()!.height - 50),
  );
  await page.mouse.wheel(0, delta);
}

for (const theme of ["light", "dark"] as const)
  for (const [route, label, name] of [
    ["skills", "已安装 Skills 列表", "Scroll Skill 39"],
    ["mcp", "MCP 列表", "Scroll MCP 39"],
  ] as const) {
    test(`${route} ${theme} installed list is reachable by wheel and keyboard, not programmatic scrolling`, async ({
      page,
    }, info) => {
      await installLongFixture(page);
      await page.addInitScript(
        (theme) => localStorage.setItem("fyagent-theme", theme),
        theme,
      );
      const health = monitorPageHealth(page);
      await openRendererPage(page, `/${route}`);
      const area = page.getByRole("region", { name: label, exact: true });
      const tail = area.getByRole("button", { name: new RegExp(name) });
      await expect(area).toBeVisible();
      await info.attach("scroll-chain", {
        body: JSON.stringify(
          await area.evaluate((node) => {
            const chain = [];
            for (
              let n: HTMLElement | null = node as HTMLElement;
              n;
              n = n.parentElement
            ) {
              chain.push({
                name: n.className,
                height: n.clientHeight,
                scroll: n.scrollHeight,
                overflow: getComputedStyle(n).overflowY,
              });
            }
            return chain;
          }),
        ),
        contentType: "application/json",
      });
      await expect(tail).not.toBeInViewport();
      await wheelInside(page, area, 12_000);
      await expect(tail).toBeInViewport();
      await wheelInside(page, area, -12_000);
      const first = area.getByRole("button").first();
      await expect(first).toBeInViewport();
      await first.focus();
      await page.keyboard.press("End");
      await expect(tail).toBeInViewport();
      await page.getByRole("tab", { name: "发现", exact: true }).click();
      await page.getByRole("tab", { name: "已安装", exact: true }).click();
      await wheelInside(page, area, 12_000);
      await expect(tail).toBeInViewport();
      await expectHealthyPage(page, health);
    });
  }

test("Skills discovery keeps its own bounded scroll region after tab switches", async ({
  page,
}) => {
  await installLongFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/skills");
  for (let visit = 0; visit < 2; visit++) {
    await page.getByRole("tab", { name: "发现", exact: true }).click();
    const area = page.locator(".fy-feature-discovery-scroll");
    const tail = page.getByText("Market Skill 19", { exact: true });
    await expect(tail).toBeAttached();
    await wheelInside(page, area, 18_000);
    await expect(tail).toBeInViewport();
    await page.getByRole("tab", { name: "已安装", exact: true }).click();
    await expect(
      page.getByRole("region", { name: "已安装 Skills 列表", exact: true }),
    ).toBeVisible();
  }
  await expectHealthyPage(page, health);
});

for (const route of [
  "agents",
  "auth",
  "models?target=workbuddy",
  "prompts",
  "memory",
]) {
  test(`${route} has a reachable native scroll owner for its populated content`, async ({
    page,
  }, info) => {
    await installLongFixture(page);
    await openRendererPage(page, `/${route}`);
    const root = page.getByTestId(`${route.split("?")[0]}-page`);
    await expect(root).toBeVisible();
    if (route.startsWith("models")) {
      await root
        .getByRole("button", { name: /当前已有的第三方模型 ID/ })
        .click();
      await expect(
        root.getByText("fixture-model-79", { exact: true }),
      ).toBeAttached();
      await expect
        .poll(() =>
          root
            .locator(".fy-collapsible-panel")
            .first()
            .evaluate((n) => (n as HTMLElement).style.height),
        )
        .toBe("auto");
    }
    // Read layout to choose an actual overflowing owner. Never change scrollTop
    // or force auto-positioning; the only movement comes from physical wheel.
    // Flow pages deliberately scroll their ancestor viewport; include it,
    // rather than incorrectly requiring every route to add a nested scroller.
    const owners = page.locator(
      '[data-testid="content-viewport"], [data-testid="content-viewport"] *',
    );
    const eligible = async () =>
      owners.evaluateAll((nodes) =>
        nodes
          .map((node, index) => {
            const element = node as HTMLElement;
            const b = element.getBoundingClientRect();
            return {
              index,
              name: element.className,
              overflow: getComputedStyle(element).overflowY,
              excess: element.scrollHeight - element.clientHeight,
              top: b.top,
              bottom: b.bottom,
              width: b.width,
              height: b.height,
            };
          })
          .filter(
            (b) =>
              /auto|scroll/.test(b.overflow) &&
              b.excess > 80 &&
              b.width > 30 &&
              b.height > 60 &&
              b.top < innerHeight - 70 &&
              b.bottom > 140,
          ),
      );
    await expect.poll(async () => (await eligible()).length).toBeGreaterThan(0);
    const candidates = await eligible();
    const owner = owners.nth(candidates.at(-1)!.index);
    await info.attach("native-scroll-owner", {
      body: JSON.stringify(candidates),
      contentType: "application/json",
    });
    await wheelInside(page, owner, 100_000);
    await expect
      .poll(() => owner.evaluate((n) => n.scrollTop))
      .toBeGreaterThan(20);
    await expect
      .poll(() =>
        owner.evaluate((n) =>
          Math.abs(n.scrollHeight - n.clientHeight - n.scrollTop),
        ),
      )
      .toBeLessThan(2);
    await wheelInside(page, owner, -100_000);
    await expect.poll(() => owner.evaluate((n) => n.scrollTop)).toBeLessThan(2);
  });
}
