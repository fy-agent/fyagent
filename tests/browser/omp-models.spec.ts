import { expect, test } from "@playwright/test";
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

// Browser/IPC fixture contracts. Real YAML parsing, disk writes and omp startup
// belong to native and CLI acceptance; this suite does not replace them.
test("omp directory entry reads an existing provider, previews, saves and rereads", async ({
  page,
}, testInfo) => {
  await installRichTauriFeatureFixture(page, {
    firstUseGuideState: "dismissed",
  });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/agents");
  const directory = page.getByRole("region", { name: "AI 软件目录" });
  const ompCard = directory.locator('[data-agent-id="omp"]');
  await expect(ompCard).toContainText("Oh My Pi (omp)");
  await ompCard.getByRole("button", { name: "进行配置", exact: true }).click();
  await expect(page).toHaveURL(/#\/agents\?target=omp&section=models$/);
  const configuration = page.getByRole("region", {
    name: "Oh My Pi (omp) 配置",
    exact: true,
  });
  await expect(
    configuration.getByText("Model A", { exact: true }),
  ).toBeVisible();
  await expect(configuration).toContainText("omp 默认模型 · fyagent-gateway");
  await configuration
    .getByRole("button", { name: "管理模型", exact: true })
    .click();
  await expect(page).toHaveURL(
    /#\/models\?target=omp&agentReturn=omp&agentSection=models$/,
  );

  const panel = page.getByRole("region", {
    name: "Oh My Pi 模型配置",
    exact: true,
  });
  await expect(panel.getByTestId("omp-status")).toContainText(
    "已安装 · v18.12.5",
  );
  await expect(
    panel.getByRole("combobox", { name: "供应商", exact: true }),
  ).toHaveValue("existing:fyagent-gateway");
  await expect(panel.getByLabel("服务地址", { exact: true })).toHaveValue(
    "https://gateway.example/v1",
  );
  await expect(
    panel.getByRole("button", { name: "保留现有引用", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  const previewButton = panel.getByRole("button", {
    name: "预览并保存",
    exact: true,
  });
  await expect(previewButton).toBeEnabled();
  await expect(previewButton).toBeInViewport();
  await expectNoHorizontalOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("omp-editor-entry.png") });

  await panel
    .getByLabel("服务地址", { exact: true })
    .fill("https://updated.example/v1");
  await panel
    .getByRole("combobox", { name: "协议", exact: true })
    .selectOption("openai-responses");
  await panel.getByLabel("模型 ID", { exact: true }).fill("vendor/model-b");
  await panel.getByLabel("显示名称（可选）", { exact: true }).fill("Model B");
  await panel.getByRole("button", { name: "添加", exact: true }).click();
  await panel
    .getByRole("checkbox", { name: "同时设为 omp 默认模型", exact: true })
    .check();
  await panel
    .getByRole("combobox", { name: "默认模型", exact: true })
    .selectOption("vendor/model-b");
  await previewButton.click();

  const dialog = page.getByRole("dialog", { name: "保存前确认", exact: true });
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByLabel("将修改的文件").getByRole("listitem"),
  ).toHaveCount(2);
  await expect(dialog).toContainText("~/.omp/agent/models.yml");
  await expect(dialog).toContainText("~/.omp/agent/config.yml");
  await expect(dialog).toContainText("fyagent-gateway/vendor/model-b");
  await expect(dialog).toContainText("https://updated.example/v1");
  await expect(dialog).toContainText("环境变量或密钥引用");
  const beforeApply = await featureFixtureCalls(page);
  expect(
    beforeApply.filter((call) => call.command === "apply_omp_save"),
  ).toEqual([]);
  expect(
    beforeApply.filter((call) => call.command === "preview_omp_save"),
  ).toEqual([
    {
      command: "preview_omp_save",
      payload: {
        plan: {
          provider: {
            id: "fyagent-gateway",
            baseUrl: "https://updated.example/v1",
            api: "openai-responses",
            apiKey: null,
            auth: null,
            models: [
              { id: "vendor/model-b", name: "Model B" },
              { id: "vendor/model-a", name: "Model A" },
            ],
          },
          defaultModel: "fyagent-gateway/vendor/model-b",
          expectedModelsRevision: "fixture-omp-models-1",
          expectedConfigRevision: "fixture-omp-config-1",
        },
      },
    },
  ]);
  const confirmButton = dialog.getByRole("button", {
    name: "确认保存",
    exact: true,
  });
  await expect(confirmButton).toBeInViewport();
  await page.screenshot({
    path: testInfo.outputPath("omp-preview-confirmation.png"),
  });
  await confirmButton.click();
  await expect(dialog).toHaveCount(0);
  await expect(
    page.getByText("已保存到 omp 配置", { exact: true }),
  ).toBeVisible();
  await expect(panel.getByTestId("omp-status")).toContainText(
    "fyagent-gateway/vendor/model-b",
  );
  await expect(panel.getByLabel("服务地址", { exact: true })).toHaveValue(
    "https://updated.example/v1",
  );
  await expect(
    panel.getByRole("combobox", { name: "协议", exact: true }),
  ).toHaveValue("openai-responses");
  await expect(panel.getByLabel("omp 保存结果")).toContainText(
    "config：~/.omp/agent/config.yml 已写入",
  );
  const afterApply = await featureFixtureCalls(page);
  expect(
    afterApply.filter((call) => call.command === "apply_omp_save"),
  ).toEqual([
    {
      command: "apply_omp_save",
      payload: {
        plan: {
          planId: "omp-plan:fixture",
          planDigest: "fixture-omp-plan-digest",
        },
      },
    },
  ]);
  const applyIndex = afterApply.findIndex(
    (call) => call.command === "apply_omp_save",
  );
  expect(
    afterApply
      .slice(applyIndex + 1)
      .some((call) => call.command === "read_omp_config"),
  ).toBe(true);

  // A second preview proves the renderer now uses the reread revision and models.
  await previewButton.click();
  await expect(dialog).toBeVisible();
  const previews = (await featureFixtureCalls(page)).filter(
    (call) => call.command === "preview_omp_save",
  );
  expect(previews).toHaveLength(2);
  expect(previews[1].payload.plan).toMatchObject({
    expectedModelsRevision: "fixture-omp-models-2",
    expectedConfigRevision: "fixture-omp-config-2",
    provider: {
      baseUrl: "https://updated.example/v1",
      api: "openai-responses",
      models: expect.arrayContaining([
        { id: "vendor/model-a", name: "Model A" },
        { id: "vendor/model-b", name: "Model B" },
      ]),
    },
  });
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "apply_omp_save",
    ),
  ).toHaveLength(1);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("omp cancelling a preview preserves the draft without writing", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=omp");
  const baseUrl = page.getByLabel("服务地址", { exact: true });
  await expect(baseUrl).toHaveValue("https://gateway.example/v1");
  await baseUrl.fill("https://draft.example/v1");
  await page.getByRole("button", { name: "预览并保存", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "保存前确认", exact: true });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(baseUrl).toHaveValue("https://draft.example/v1");
  expect(
    (await featureFixtureCalls(page)).filter(
      (call) => call.command === "apply_omp_save",
    ),
  ).toEqual([]);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("omp missing CLI shows installation guidance while configuration stays editable", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, { ompCliDetected: false });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=omp");
  await expect(page.getByTestId("omp-status")).toContainText("未检测到");
  await expect(
    page.getByText("irm https://omp.sh/install.ps1 | iex", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "打开 omp 官方安装说明", exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("服务地址", { exact: true })).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "预览并保存", exact: true }),
  ).toBeEnabled();
  expect(
    (await featureFixtureCalls(page)).filter((call) =>
      ["preview_omp_save", "apply_omp_save"].includes(call.command),
    ),
  ).toEqual([]);
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

for (const file of ["models", "config"] as const) {
  test(`omp invalid ${file} YAML shows a repair action and prevents saving`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page, { ompInvalidYaml: file });
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/models?target=omp");
    await expect(
      page.getByText("omp 配置文件无法解析", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "预览并保存", exact: true }),
    ).toBeDisabled();
    await expect(page.getByLabel("服务地址", { exact: true })).toHaveCount(0);
    const beforeReads = (await featureFixtureCalls(page)).filter(
      (call) => call.command === "read_omp_config",
    ).length;
    await page.getByRole("button", { name: "重新读取", exact: true }).click();
    await expect
      .poll(
        async () =>
          (await featureFixtureCalls(page)).filter(
            (call) => call.command === "read_omp_config",
          ).length,
      )
      .toBeGreaterThan(beforeReads);
    await expect(
      page.getByText("omp 配置文件无法解析", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "重新读取", exact: true }),
    ).toBeEnabled();
    expect(
      (await featureFixtureCalls(page)).filter((call) =>
        ["preview_omp_save", "apply_omp_save"].includes(call.command),
      ),
    ).toEqual([]);
    await expectNoHorizontalOverflow(page);
    await expectHealthyPage(page, health);
  });
}
