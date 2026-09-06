import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  monitorPageHealth,
  openRendererPage,
} from "./support";

for (const flow of ["discovery", "editor"] as const) {
  test(`MCP ${flow} hands its actual submit source to the asynchronous trust notice`, async ({
    page,
  }) => {
    await installRichTauriFeatureFixture(page);
    await page.addInitScript(() => {
      const writes: Record<string, unknown> = {};
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, payload = {}) => {
        if (command === "upsert_mcp_server") {
          const server = payload.server as { id: string };
          window.__FYAGENT_FEATURE_FIXTURE__.calls.push({ command, payload });
          writes[server.id] = structuredClone(server);
          return;
        }
        const result = await invoke(command, payload);
        return command === "get_mcp_servers"
          ? { ...(result as Record<string, unknown>), ...writes }
          : result;
      };
    });
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/mcp");
    if (flow === "discovery") {
      await page.getByRole("tab", { name: "发现", exact: true }).click();
      const card = page.locator("article").filter({
        has: page.getByRole("heading", { name: "Time", exact: true }),
      });
      await card.getByRole("button", { name: "安装", exact: true }).click();
      await page
        .getByRole("dialog")
        .getByRole("radio", { name: "WorkBuddy", exact: true })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "下一步", exact: true })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "确认安装", exact: true })
        .click();
    } else {
      await page
        .getByRole("button", { name: "添加 MCP", exact: true })
        .first()
        .click();
      const editor = page.getByRole("dialog", {
        name: "添加 MCP",
        exact: true,
      });
      await editor
        .getByRole("textbox", { name: "ID", exact: true })
        .fill("fixture-origin-save");
      await editor
        .getByRole("textbox", { name: "名称", exact: true })
        .fill("Origin Save Fixture");
      await editor
        .getByRole("textbox", { name: "命令", exact: true })
        .fill("fixture-command");
      await editor
        .getByRole("switch", { name: "WorkBuddy MCP 分配", exact: true })
        .check();
      await editor.getByRole("button", { name: "保存", exact: true }).click();
    }
    const notice = page.getByRole("dialog", {
      name: "需要在 WorkBuddy 中信任 MCP",
    });
    await expect(notice).toHaveAttribute("data-motion-origin", "trigger");
    await expect(notice).toHaveAttribute("data-motion-settled", "true");
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            window.__FYAGENT_FEATURE_FIXTURE__.calls.filter(
              (call) => call.command === "upsert_mcp_server",
            ).length,
        ),
      )
      .toBe(1);
    await notice.getByRole("button", { name: "知道了" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expectHealthyPage(page, health);
  });
}
