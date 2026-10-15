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

test("OpenCode refreshes a saved connection and saves only the selected new model", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page, { opencodeSavedProvider: true });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/models?target=opencode");

  const panel = page.getByRole("region", { name: "OpenCode 模型设置" });
  await expect(
    panel.getByRole("combobox", { name: "供应商", exact: true }),
  ).toHaveValue("existing:vibekey");
  await expect(panel.getByLabel("服务地址", { exact: true })).toHaveValue("");
  await expect(panel.getByLabel("API Key", { exact: true })).toHaveValue("");
  await panel.getByRole("button", { name: "拉取模型", exact: true }).click();

  const candidates = panel.getByRole("region", { name: "可添加的模型 ID" });
  const newModel = candidates.getByRole("button", {
    name: "grok-4.7",
    exact: true,
  });
  await expect(newModel).toHaveAttribute("aria-pressed", "false");
  await expect(
    candidates.getByRole("button", { name: "grok-4.6", exact: true }),
  ).toHaveCount(0);
  expect(
    (await featureFixtureCalls(page))
      .filter((call) => call.command === "fetch_opencode_provider_models")
      .map((call) => call.payload),
  ).toEqual([
    {
      request: {
        providerId: "vibekey",
        expectedRevision: "fixture-vibekey-revision-1",
        baseUrl: "",
        apiKey: "",
        allowNoApiKey: false,
      },
    },
  ]);
  expect(
    (await featureFixtureCalls(page)).some(
      (call) => call.command === "save_opencode_models",
    ),
  ).toBe(false);

  await newModel.click();
  await expect(newModel).toHaveAttribute("aria-pressed", "true");
  await expect(
    candidates.getByRole("button", { name: "grok-imagine-image", exact: true }),
  ).toHaveAttribute("aria-pressed", "false");
  await expect(panel.getByTestId("opencode-draft-models")).toContainText(
    "grok-4.7",
  );
  await expectNoHorizontalOverflow(page);

  await panel.getByRole("button", { name: "保存并应用", exact: true }).click();
  const confirmation = page.getByRole("dialog", { name: "保存前确认" });
  await expect(confirmation).toBeVisible();
  await confirmation
    .getByRole("button", { name: "确认保存", exact: true })
    .click();
  await expect(
    panel.getByText("OpenCode 模型配置已保存", { exact: true }),
  ).toBeVisible();
  const writes = (await featureFixtureCalls(page)).filter(
    (call) => call.command === "save_opencode_models",
  );
  expect(writes).toHaveLength(1);
  expect(writes[0].payload).toEqual({
    request: {
      providerId: "vibekey",
      providerName: "VibeKey",
      baseUrl: "",
      apiKey: "",
      selectedModelIds: ["grok-4.7"],
      removedModelIds: [],
      expectedRevision: "fixture-vibekey-revision-1",
    },
  });

  await panel.getByRole("button", { name: /当前已有的第三方模型 ID/ }).click();
  const existing = panel.getByRole("region", {
    name: "当前已有的第三方模型 ID",
  });
  await expect(existing.getByText("grok-4.7", { exact: true })).toBeVisible();
  await expect(existing.getByText("grok-4.6", { exact: true })).toBeVisible();
  await expect(existing.getByText("grok-4.5", { exact: true })).toBeVisible();
  await expect(
    existing.getByText("grok-imagine-image", { exact: true }),
  ).toHaveCount(0);
  await expectHealthyPage(page, health);
});
