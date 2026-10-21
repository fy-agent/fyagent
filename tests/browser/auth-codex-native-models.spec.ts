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
  type RichFeatureFixtureOptions,
} from "./support/features";

const NATIVE_MODELS_COMMAND = "managed_auth_get_codex_native_models";
const OPENAI_ACCOUNT_ID = `ma1:${"1".repeat(32)}`;

async function nativeModelCalls(page: Page) {
  return (await featureFixtureCalls(page)).filter(
    (call) => call.command === NATIVE_MODELS_COMMAND,
  );
}

async function openCodexCatalog(
  page: Page,
  scenario: RichFeatureFixtureOptions["codexNativeModelsScenario"] = "ready",
) {
  await installRichTauriFeatureFixture(page, {
    firstUseGuideState: "dismissed",
    codexNativeModelsScenario: scenario,
  });
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/auth?view=connections");
  await page.getByTestId("managed-auth-consumer-codex").click();
  const detail = page.getByRole("region", {
    name: "Codex 连接详情",
    exact: true,
  });
  const catalog = detail.getByRole("region", {
    name: "本机 Codex 模型目录",
    exact: true,
  });
  await expect(catalog).toBeVisible();
  await expect(detail).toContainText("OpenAI · browser-fixture@example.com");
  expect(await nativeModelCalls(page)).toEqual([]);
  await catalog.getByRole("button", { name: "获取模型", exact: true }).click();
  await expect.poll(async () => (await nativeModelCalls(page)).length).toBe(1);
  expect(await nativeModelCalls(page)).toEqual([
    {
      command: NATIVE_MODELS_COMMAND,
      payload: { accountId: OPENAI_ACCOUNT_ID },
    },
  ]);
  return { detail, catalog, health };
}

// Browser/IPC fixture evidence. No real account, app-server or native helper is
// started here; the native acceptance records cover those separate layers.
test("reads and refreshes the attributed Codex native catalog in connection details", async ({
  page,
}, testInfo) => {
  const { detail, catalog, health } = await openCodexCatalog(page);
  const source = catalog.getByText(
    /来源：本机 Codex 原生官方目录 · Codex CLI 0\.0\.0-fixture · 读取于/u,
  );
  await expect(source).toBeVisible();
  const models = catalog.getByRole("list").getByRole("listitem");
  await expect(models).toHaveCount(2);
  await expect(
    models.nth(0).getByText("Fixture Codex", { exact: true }),
  ).toBeVisible();
  await expect(
    models.nth(0).getByText("gpt-fixture-codex", { exact: true }),
  ).toBeVisible();
  await expect(models.nth(0).getByText("默认", { exact: true })).toBeVisible();
  await expect(models.nth(1)).toHaveText("gpt-fixture-mini");
  await expect(catalog).toContainText("与订阅转发的模型目录分开");
  // The native login catalog is not the current third-party request provider.
  await expect(detail).toContainText("DeepSeek API");

  const refresh = catalog.getByRole("button", {
    name: "重新获取模型",
    exact: true,
  });
  await expect(refresh).toBeEnabled();
  await refresh.click();
  await expect.poll(async () => (await nativeModelCalls(page)).length).toBe(2);
  expect(await nativeModelCalls(page)).toEqual([
    {
      command: NATIVE_MODELS_COMMAND,
      payload: { accountId: OPENAI_ACCOUNT_ID },
    },
    {
      command: NATIVE_MODELS_COMMAND,
      payload: { accountId: OPENAI_ACCOUNT_ID },
    },
  ]);
  await expect(source).toBeVisible();
  await expect(models).toHaveCount(2);
  await expect(refresh).toBeEnabled();
  await catalog.scrollIntoViewIfNeeded();
  await expect(refresh).toBeInViewport();
  await refresh.click({ trial: true });
  await expect(source).toBeInViewport();
  await expect(models.nth(1)).toBeInViewport();
  await expectNoHorizontalOverflow(page);
  const viewport = page.viewportSize()!;
  await page.screenshot({
    path: testInfo.outputPath(
      `a5-codex-native-models-${viewport.width}x${viewport.height}-fixture.png`,
    ),
  });
  await expectHealthyPage(page, health);
});

test("explains a different native ChatGPT account without showing its catalog", async ({
  page,
}) => {
  const { catalog, health } = await openCodexCatalog(page, "other_account");
  await expect(catalog).toContainText(
    "本机 Codex 当前登录的是另一个 ChatGPT 账号",
  );
  await expect(catalog).toContainText("把此账号连接到 Codex 后再获取");
  await expect(catalog.getByRole("list")).toHaveCount(0);
  await expect(catalog.getByText(/来源：本机 Codex 原生官方目录/u)).toHaveCount(
    0,
  );
  await expect(
    catalog.getByRole("button", { name: "重新获取模型", exact: true }),
  ).toBeEnabled();
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});

test("gives a file-store recovery path when native credential attribution is unsupported", async ({
  page,
}) => {
  const { catalog, health } = await openCodexCatalog(
    page,
    "unsupported_credential_store",
  );
  const status = catalog;
  await expect(status).toContainText("系统凭据库");
  await expect(status).toContainText("file");
  await expect(status).toContainText(/重新登录|再次登录/u);
  await expect(catalog.getByRole("list")).toHaveCount(0);
  await expect(catalog.getByText(/来源：本机 Codex 原生官方目录/u)).toHaveCount(
    0,
  );
  await expect(
    catalog.getByRole("button", { name: "重新获取模型", exact: true }),
  ).toBeEnabled();
  await expect(status).not.toContainText("无法识别返回的账号状态");
  await expectNoHorizontalOverflow(page);
  await expectHealthyPage(page, health);
});
