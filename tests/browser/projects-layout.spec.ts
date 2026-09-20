import { expect, test } from "@playwright/test";
import { writeFile } from "node:fs/promises";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

test("selected project fills its workspace and keeps preparation reachable by wheel", async ({
  page,
}, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const projectId = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const customerId = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
    const project = {
      projectId,
      customerId,
      name: "经营周报项目",
      projectRevision: 1,
      archived: false,
      resources: [],
      credentials: [],
      kit: null,
      contextGeneration: null,
      codexPrepared: false,
      createdAt: "2026-09-19T00:00:00Z",
      updatedAt: "2026-09-19T00:00:00Z",
    };
    window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
      if (command === "projects_list_customers")
        return [{ customerId, name: "星河商贸", revision: 1, archived: false }];
      if (command === "projects_list") return [project];
      if (command === "projects_get_context")
        return {
          projectId,
          projectRevision: 1,
          content:
            "整理本周经营数据，核对增长率与目标完成率，并形成可交接的周报。",
          directory: null,
          state: "materialized",
          codexInstructions: null,
        };
      if (
        command === "projects_resource_options" ||
        command === "projects_credential_options"
      )
        return [];
      if (command === "projects_dependency_snapshot")
        return {
          projectId,
          projectRevision: 1,
          archived: false,
          kit: null,
          kitState: "missing",
          resources: [],
          credentials: [],
          contextGeneration: null,
          contextState: "materialized",
          observedAt: "2026-09-19T00:00:00Z",
          runtimeAvailable: false,
        };
      return invoke(command, payload);
    };
  });
  const health = monitorPageHealth(page);
  await openRendererPage(
    page,
    "/projects?project=aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  );
  await expect(
    page.getByRole("textbox", { name: "工作说明", exact: true }),
  ).toHaveValue(/整理本周经营数据/);
  const panel = page.getByRole("tabpanel", { name: "项目准备", exact: true });
  const measurements = await panel.evaluate((node) => {
    if (!(node instanceof HTMLElement))
      throw new Error("Project panel is not an HTML element");
    const chain = [];
    for (
      let element: HTMLElement | null = node;
      element;
      element = element.parentElement
    ) {
      const rect = element.getBoundingClientRect();
      chain.push({
        name: element.className,
        top: rect.top,
        height: rect.height,
        scrollHeight: element.scrollHeight,
        overflow: getComputedStyle(element).overflowY,
      });
    }
    return chain;
  });
  const measurementsPath = info.outputPath("workspace-height-chain.json");
  await writeFile(measurementsPath, JSON.stringify(measurements, null, 2));
  await info.attach("project-workspace-height-chain", {
    path: measurementsPath,
    contentType: "application/json",
  });
  const screenshotPath = info.outputPath("project-preparation.png");
  await page.screenshot({ path: screenshotPath });
  await info.attach("project-preparation", {
    path: screenshotPath,
    contentType: "image/png",
  });
  expect(measurements[0].height).toBeGreaterThanOrEqual(
    page.viewportSize()!.height >= 700 ? 250 : 150,
  );
  await expectNoHorizontalOverflow(page);
  await expect
    .poll(() =>
      page
        .locator(".fy-projects-detail")
        .evaluate((node) => node.scrollWidth - node.clientWidth),
    )
    .toBeLessThanOrEqual(1);

  const box = await panel.boundingBox();
  if (!box) throw new Error("Project preparation has no visible scroll area");
  await page.mouse.move(box.x + box.width - 12, box.y + box.height / 2);
  await page.mouse.wheel(0, 4000);
  await expect
    .poll(() => panel.evaluate((node) => node.scrollTop))
    .toBeGreaterThan(20);
  await expect(page.getByText("项目设置", { exact: true })).toBeInViewport();
  await page.mouse.wheel(0, -4000);
  await expect
    .poll(() => panel.evaluate((node) => node.scrollTop))
    .toBeLessThan(2);
  await expect(
    page.getByRole("textbox", { name: "工作说明", exact: true }),
  ).toBeInViewport();
  await expectHealthyPage(page, health);
});
