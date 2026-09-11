import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import type { AgentHealthSnapshot } from "../../src/shared/features/health";
import { expectNoHorizontalOverflow } from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

async function healthCalls(page: Page) {
  return (await featureFixtureCalls(page))
    .filter((call) => call.command === "get_agent_health")
    .map((call) => call.payload.agentId);
}

test("selection lens follows the selected Agent through status reordering and clipped scrolling", async ({
  page,
}, info) => {
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/health?agent=codex");
  const detail = page.getByRole("region", { name: "Codex 运行状态" });
  const rail = page.getByRole("region", { name: "软件运行状态" });
  const selected = page.getByTestId("health-agent-codex");
  const lens = rail.getByTestId("selection-lens");
  await expect(detail.locator(".fy-health-status")).toHaveText("本机检查正常");
  const buttonNode = await selected.elementHandle();
  const lensNode = await lens.elementHandle();
  expect(buttonNode).not.toBeNull();
  expect(lensNode).not.toBeNull();
  const expectAligned = async () => {
    await expect
      .poll(() =>
        rail.evaluate((node) => {
          const host = node.querySelector('[aria-current="true"]')!;
          const pill = node.querySelector(".fy-selection-lens")!;
          const a = host.getBoundingClientRect();
          const b = pill.getBoundingClientRect();
          return Math.max(
            Math.abs(a.x - b.x),
            Math.abs(a.y - b.y),
            Math.abs(a.width - b.width),
            Math.abs(a.height - b.height),
          );
        }),
      )
      .toBeLessThanOrEqual(1);
    expect(
      await selected.evaluate(
        (node, original) => node === original,
        buttonNode,
      ),
    ).toBe(true);
    expect(
      await lens.evaluate((node, original) => node === original, lensNode),
    ).toBe(true);
    await expect(selected).toHaveAttribute("aria-current", "true");
  };
  await selected.focus();
  await expectAligned();

  // One real port read observes a damaged configuration; the following read
  // observes its restoration. The production adapter still validates both.
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    let codexReads = 0;
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      const result = await invoke(command, payload);
      if (command !== "get_agent_health" || payload?.agentId !== "codex")
        return result;
      codexReads += 1;
      if (codexReads !== 1) return result;
      const snapshot = result as AgentHealthSnapshot;
      return {
        ...snapshot,
        checks: snapshot.checks.map((check) =>
          check.id === "configuration"
            ? {
                ...check,
                state: "blocked",
                severity: "error",
                reasonCode: "configuration_unreadable",
                action: "configuration",
              }
            : check,
        ),
      };
    };
    window.__FYAGENT_FEATURE_FIXTURE__.holdHealth();
  });
  await detail.getByRole("button", { name: "重新检查此软件" }).click();
  await expect.poll(() => healthCalls(page)).toEqual(["codex", "codex"]);
  await selected.focus();
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.releaseHealth());
  await expect(detail.locator(".fy-health-status")).toHaveText("暂不可用");
  await expect(
    rail.getByRole("listitem").first().getByRole("button"),
  ).toHaveAttribute("data-testid", "health-agent-codex");
  await expect(selected).toBeFocused();
  await expectAligned();

  // Clip half the selected row at the real rail scroll boundary. The pill
  // must track its host while the overflow owner clips both descendants.
  // A tall project viewport can fit all seven rows; shrink its height through
  // the actual window layout so this part always exercises real overflow.
  await page.setViewportSize({ ...page.viewportSize()!, height: 600 });
  await rail.evaluate((node) => {
    const host = node.querySelector('[aria-current="true"]')!;
    const row = host.getBoundingClientRect();
    node.scrollTop +=
      row.top - node.getBoundingClientRect().top + row.height / 2;
  });
  await expectAligned();
  await expect
    .poll(() =>
      rail.evaluate((node) => {
        const host = node.querySelector('[aria-current="true"]')!;
        const row = host.getBoundingClientRect();
        const clip = node.getBoundingClientRect();
        const x = row.left + row.width / 2;
        return {
          scrollable: node.scrollHeight > node.clientHeight,
          overflow: getComputedStyle(node).overflowY,
          partial: row.top < clip.top && row.bottom > clip.top,
          outsideClipped: !host.contains(
            document.elementFromPoint(x, clip.top - 2),
          ),
          insideReachable: host.contains(
            document.elementFromPoint(x, clip.top + 4),
          ),
        };
      }),
    )
    .toEqual({
      scrollable: true,
      overflow: "auto",
      partial: true,
      outsideClipped: true,
      insideReachable: true,
    });
  await info.attach("health-selection-clipped", {
    body: await rail.screenshot({
      path: info.outputPath("health-selection-clipped.png"),
    }),
    contentType: "image/png",
  });

  await detail.locator('[data-check-id="configuration"] button').click();
  await expect(page).toHaveURL(/#\/models\?target=codex$/u);
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.holdHealth());
  const returnLink = page.getByRole("link", { name: "运行状态", exact: true });
  await returnLink.click();
  await expect
    .poll(() => healthCalls(page))
    .toEqual(["codex", "codex", "codex"]);
  await returnLink.focus();
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.releaseHealth());
  await expect(detail.locator(".fy-health-status")).toHaveText("本机检查正常");
  await expect(
    rail.getByRole("listitem").last().getByRole("button"),
  ).toHaveAttribute("data-testid", "health-agent-codex");
  await expect(returnLink).toBeFocused();
  await expectAligned();
  await selected.scrollIntoViewIfNeeded();
  await expectAligned();
  await info.attach("health-selection-restored", {
    body: await rail.screenshot({
      path: info.outputPath("health-selection-restored.png"),
    }),
    contentType: "image/png",
  });
});

test("health reads one target, opens model configuration without testing, and rereads on return", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/health?agent=codex");
  const detail = page.getByRole("region", { name: "Codex 运行状态" });
  await expect(detail.getByText("本机检查正常", { exact: true })).toBeVisible();
  await expect(detail.locator("[data-check-id]")).toHaveCount(12);
  expect(await healthCalls(page)).toEqual(["codex"]);
  const initial = await featureFixtureCalls(page);
  expect(
    initial.filter((call) =>
      /get_agent_auth_observation|get_agent_install_readiness|fetch_models|check_model|start_agent|managed_auth_start/u.test(
        call.command,
      ),
    ),
  ).toEqual([]);
  await detail.locator(".fy-health-group").first().hover();
  await page.mouse.wheel(0, 2500);
  const testEntry = detail.getByRole("button", {
    name: "前往模型测试：最近一次请求",
  });
  await expect(testEntry).toBeInViewport();
  await testEntry.click();
  await expect(page).toHaveURL(/#\/models\?target=codex$/u);
  await expect(
    page.getByRole("region", { name: "Codex 模型配置" }),
  ).toBeVisible();
  expect(
    (await featureFixtureCalls(page)).filter((call) =>
      /check_model|check_reachability|probe_model|test_model_endpoint/u.test(
        call.command,
      ),
    ),
  ).toEqual([]);
  await page.getByRole("link", { name: "运行状态", exact: true }).click();
  await expect(detail).toBeVisible();
  await expect.poll(() => healthCalls(page)).toEqual(["codex", "codex"]);
});

test("batch preserves good snapshots after a failed reread and supports filtering", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/health?agent=codex");
  await expect(
    page.getByRole("button", { name: "检查全部软件" }),
  ).toBeEnabled();
  await expect(
    page
      .getByRole("region", { name: "Codex 运行状态" })
      .locator("[data-check-id]"),
  ).toHaveCount(12);
  await page.evaluate(() =>
    window.__FYAGENT_FEATURE_FIXTURE__.failHealth("codex"),
  );
  await page.getByRole("button", { name: "检查全部软件" }).click();
  await expect(
    page.getByText("检查完成 · 6 个已更新，1 个读取失败，可单独重试"),
  ).toBeVisible();
  await expect(page.getByText("已检查 7 / 7")).toBeVisible();
  await expect(
    page.getByText("本次读取失败，以下保留上次结果。请重新检查。"),
  ).toBeVisible();
  await expect(
    page
      .getByRole("region", { name: "Codex 运行状态" })
      .locator("[data-check-id]"),
  ).toHaveCount(12);
  await page
    .getByRole("combobox", { name: "筛选状态" })
    .selectOption("needs_attention");
  await expect(page.getByTestId("health-agent-claude-code")).toBeVisible();
  await expect(page.getByTestId("health-agent-codex")).toHaveCount(0);
  await page.getByRole("searchbox", { name: "搜索软件" }).fill("missing-agent");
  await expect(page.getByText("没有匹配的软件")).toBeVisible();
  await page.getByRole("button", { name: "清除筛选" }).click();
  await expect(page.getByTestId("health-agent-codex")).toBeVisible();
  await page.evaluate(() =>
    window.__FYAGENT_FEATURE_FIXTURE__.failHealth(null),
  );
  await page.getByRole("button", { name: "重新检查此软件" }).click();
  await expect(
    page
      .getByRole("region", { name: "Codex 运行状态" })
      .getByText("本机检查正常", { exact: true }),
  ).toBeVisible();
});

test("stop and route hiding revoke pending batch dispatch", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/health?agent=codex");
  await expect(
    page
      .getByRole("region", { name: "Codex 运行状态" })
      .locator("[data-check-id]"),
  ).toHaveCount(12);
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.holdHealth());
  await page.getByRole("button", { name: "检查全部软件" }).click();
  await expect.poll(() => healthCalls(page)).toEqual(["codex", "qoderwork"]);
  await page.getByRole("button", { name: "停止检查" }).click();
  await expect(page.getByText("正在完成当前检查，之后停止。")).toBeVisible();
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.releaseHealth());
  await expect(page.getByText("检查已停止 · 已完成 1 / 7")).toBeVisible();
  expect(await healthCalls(page)).toEqual(["codex", "qoderwork"]);
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.holdHealth());
  await page.getByRole("button", { name: "检查全部软件" }).click();
  await expect
    .poll(() => healthCalls(page))
    .toEqual(["codex", "qoderwork", "qoderwork"]);
  await page.getByRole("link", { name: "模型管理", exact: true }).click();
  await expect(page.getByTestId("health-page")).not.toBeVisible();
  await page.evaluate(() => window.__FYAGENT_FEATURE_FIXTURE__.releaseHealth());
  await page.getByRole("link", { name: "运行状态", exact: true }).click();
  await expect
    .poll(() => healthCalls(page))
    .toEqual(["codex", "qoderwork", "qoderwork", "codex"]);
});

test("Agent directory links to the matching health detail and the login entry uses the existing flow", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await page.goto("/#/agents");
  const card = page.locator('[data-agent-id="claude-code"]');
  await card.getByRole("button", { name: "查看 Claude Code 运行状态" }).click();
  await expect(page).toHaveURL(/#\/health\?agent=claude-code$/u);
  const detail = page.getByRole("region", { name: "Claude Code 运行状态" });
  await expect(detail.locator('[data-check-id="auth"]')).toContainText(
    "本机记录显示未登录",
  );
  await detail.getByRole("button", { name: "查看登录：登录状态" }).click();
  await expect(page).toHaveURL(
    /#\/agents\?target=claude-code&section=models$/u,
  );
  await expect(page.getByTestId("agents-page")).toHaveAttribute(
    "data-view",
    "configuration",
  );
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "start_agent_auth_session",
    ),
  ).toEqual([]);
});

for (const theme of ["light", "dark"] as const) {
  test(`health remains readable and keyboard accessible in a narrow ${theme} window`, async ({
    page,
  }, info) => {
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.addInitScript(
      (preference) => localStorage.setItem("fyagent-theme", preference),
      theme,
    );
    await page.setViewportSize({ width: 616, height: 900 });
    await installRichTauriFeatureFixture(page, { healthStale: true });
    await page.goto("/#/health?agent=codex");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(
      page.getByText("检查结果已超过 5 分钟，请重新检查后再判断。"),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page);
    const search = page.getByRole("searchbox", { name: "搜索软件" });
    await search.focus();
    await page.keyboard.type("Claude");
    await page.keyboard.press("Escape");
    await expect(search).toHaveValue("");
    const candidate = page.getByTestId("health-agent-claude-code");
    await candidate.focus();
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("region", { name: "Claude Code 运行状态" }),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page);
    const scan = await new AxeBuilder({ page })
      .include('[data-testid="health-page"]')
      .withRules(["color-contrast", "button-name", "label", "heading-order"])
      .analyze();
    expect(scan.violations).toEqual([]);
    await info.attach(`health-${theme}-narrow`, {
      body: await page.screenshot(),
      contentType: "image/png",
    });
  });
}
