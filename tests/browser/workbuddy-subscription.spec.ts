import { expect, test, type Page } from "@playwright/test";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

// Renderer/mock contracts only: no upstream request, native file or device evidence.
const sources = [
  {
    provider: "OpenAI",
    label: "ChatGPT · Browser Fixture",
    accountId: `ma1:${"1".repeat(32)}`,
    modelId: "chatgpt-fixture-model",
  },
  {
    provider: "xAI",
    label: "xAI · browser-xai@example.com",
    accountId: `ma1:${"2".repeat(32)}`,
    modelId: "grok-subscription-fixture-1",
  },
] as const;
const unrelatedWrites = [
  "save_workbuddy_models",
  "create_workbuddy_save_plan",
  "bind_managed_proxy_provider",
  "bind_opencode_managed_proxy",
  "save_opencode_models",
  "set_proxy_takeover_for_app",
  "apply_provider_quick_setup_with_result",
  "apply_claude_quick_setup_preview",
  "create_codex_provider_switch_plan",
  "create_codex_provider_upsert_plan",
  "switch_provider",
];
async function preview(page: Page, source: (typeof sources)[number]) {
  await openRendererPage(page, "/models?target=workbuddy");
  await page.getByRole("button", { name: "账号订阅", exact: true }).click();
  const section = page.getByRole("region", { name: "账号订阅设置" });
  await expect(
    section.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
  ).toBeDisabled();
  await section.getByRole("radio", { name: source.label, exact: true }).check();
  if (source.provider === "xAI") {
    await section
      .getByRole("button", { name: source.modelId, exact: true })
      .click();
  } else {
    await section.getByLabel("订阅模型 ID").fill(source.modelId);
  }
  await section
    .getByRole("button", { name: "预览 WorkBuddy 订阅设置" })
    .click();
  await expect(section.getByRole("button", { name: "应用更改" })).toBeEnabled();
  await expect(
    section.getByText(/http:\/\/127.0.0.1:15721\/workbuddy\/v1/),
  ).toBeVisible();
  expect(
    (await featureFixtureCalls(page)).filter(
      (c) => c.command === "apply_change_plan",
    ),
  ).toEqual([]);
  return section;
}
async function expectIsolated(page: Page) {
  const calls = await featureFixtureCalls(page);
  expect(calls.filter((c) => unrelatedWrites.includes(c.command))).toEqual([]);
  expect(calls.filter((c) => c.command === "auth_get_status")).toEqual([]);
}
for (const source of sources) {
  test(`WorkBuddy ${source.provider}: explicit preview, confirmation and fresh source readback remain isolated from four Agents`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page);
    const health = monitorPageHealth(page);
    const section = await preview(page, source);
    await section.getByRole("button", { name: "应用更改" }).click();
    await expect(
      section.getByText("WorkBuddy 订阅配置已保存", { exact: true }),
    ).toBeVisible();
    await expect(section.getByText(/保存配置不代表对话调用成功/)).toBeVisible();
    const calls = await featureFixtureCalls(page);
    expect(
      calls
        .filter((c) => c.command === "create_workbuddy_subscription_plan")
        .map((c) => c.payload),
    ).toEqual([
      {
        request: {
          accountId: source.accountId,
          modelId: source.modelId,
          expectedRevision: "fixture-revision-1",
          expectedBindingVersion: null,
        },
      },
    ]);
    expect(
      calls
        .filter((c) => c.command === "apply_change_plan")
        .map((c) => c.payload),
    ).toEqual([
      { planId: "plan-workbuddy-subscription", planDigest: "c".repeat(64) },
    ]);
    const applyIndex = calls.findIndex(
      (c) => c.command === "apply_change_plan",
    );
    expect(
      calls
        .slice(applyIndex + 1)
        .some((c) => c.command === "get_workbuddy_subscription_source"),
    ).toBe(true);
    if (source.provider === "OpenAI")
      expect(calls.filter((c) => c.command === "get_xai_oauth_models")).toEqual(
        [],
      );
    await expectIsolated(page);
    await expectNoHorizontalOverflow(page);
    await expectHealthyPage(page, health);
  });

  test(`WorkBuddy ${source.provider}: closing preview writes nothing`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page);
    const section = await preview(page, source);
    await section.getByRole("button", { name: "关闭", exact: true }).click();
    await expect(section.getByRole("button", { name: "应用更改" })).toHaveCount(
      0,
    );
    await expect(
      section.getByText("WorkBuddy 订阅配置已保存", { exact: true }),
    ).toHaveCount(0);
    expect(
      (await featureFixtureCalls(page)).filter(
        (c) => c.command === "apply_change_plan",
      ),
    ).toEqual([]);
    await expectIsolated(page);
  });

  for (const scenario of [
    "stale",
    "readback_failed",
    "wrong_account",
    "wrong_route",
    "wrong_model",
  ] as const) {
    test(`WorkBuddy ${source.provider}: ${scenario} stays unconfirmed without cross-Agent writes`, async ({
      page,
    }) => {
      await installRichTauriFeatureFixture(page, {
        workBuddySubscription: scenario,
      });
      const section = await preview(page, source);
      await section.getByRole("button", { name: "应用更改" }).click();
      await expect(
        section.getByText("当前配置待确认", { exact: true }),
      ).toBeVisible();
      await expect(
        section.getByText("WorkBuddy 订阅配置已保存", { exact: true }),
      ).toHaveCount(0);
      await section.getByRole("button", { name: "关闭", exact: true }).click();
      await expect(
        section.getByRole("button", { name: "预览 WorkBuddy 订阅设置" }),
      ).toBeDisabled();
      expect(
        (await featureFixtureCalls(page)).filter(
          (c) => c.command === "apply_change_plan",
        ),
      ).toHaveLength(1);
      await expectIsolated(page);
    });
  }
}
