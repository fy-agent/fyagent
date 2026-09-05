import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  sampleTextContrast,
  sampleControlBoundaryContrast,
} from "./support/visual";
import { openV2Page, expectNoHorizontalOverflow } from "./support";
import { mkdir } from "node:fs/promises";
import path from "node:path";

test("keeps actual text readable on blended surfaces and dialogs", async ({
  page,
}, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await installRichTauriFeatureFixture(page);
  for (const route of [
    "models?target=codex",
    "auth",
    "skills",
    "mcp",
    "agents",
    "prompts",
    "memory",
  ]) {
    await openV2Page(page, `/${route}`);
    const id = route.split("?")[0];
    await expect(page.getByTestId(`${id}-page`)).toBeVisible();
    const scope = `[data-testid="${id}-page"]`;
    const scan = await new AxeBuilder({ page })
      .include(scope)
      .withRules(["color-contrast", "button-name", "label"])
      .analyze();
    await info.attach(`axe-${id}`, {
      body: JSON.stringify(scan),
      contentType: "application/json",
    });
    expect
      .soft(
        scan.violations.map(({ id, nodes }) => ({
          id,
          nodes: nodes.map((node) => node.failureSummary),
        })),
      )
      .toEqual([]);
    const samples = await sampleTextContrast(page, scope);
    expect(samples.length).toBeGreaterThan(3);
    await info.attach(`contrast-${id}`, {
      body: JSON.stringify(samples),
      contentType: "application/json",
    });
    expect.soft(samples.filter((sample) => sample.ratio < 4.5)).toEqual([]);
    if (id === "models") {
      const boundaries = await sampleControlBoundaryContrast(
        page,
        `${scope} .fy-control-input:not(:disabled)`,
      );
      expect(boundaries.length).toBeGreaterThan(0);
      await info.attach("input-boundaries", {
        body: JSON.stringify(boundaries),
        contentType: "application/json",
      });
      expect.soft(boundaries.filter((sample) => sample.ratio < 3)).toEqual([]);
      const navigation = await sampleTextContrast(page, ".fy-side-navigation");
      expect
        .soft(navigation.filter((sample) => sample.ratio < 4.5))
        .toEqual([]);
      await page.getByRole("button", { name: "管理 Codex 账号与来源" }).hover();
      const hovered = await sampleTextContrast(page, ".fy-models-source-entry");
      expect.soft(hovered.filter((sample) => sample.ratio < 4.5)).toEqual([]);
    }
  }
  await openV2Page(page, "/auth");
  await page
    .getByRole("button", { name: "添加账号", exact: true })
    .first()
    .click();
  await expect(page.getByRole("dialog")).toBeVisible();
  const samples = await sampleTextContrast(page, ".fy-control-dialog");
  expect.soft(samples.filter((sample) => sample.ratio < 4.5)).toEqual([]);
  await info.attach("dialog.png", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
  const directory = path.resolve("node_modules/.cache/fyagent-ux-round4");
  await mkdir(directory, { recursive: true });
  await page.screenshot({
    path: path.join(directory, `${info.project.name}-dialog.png`),
  });
  await expectNoHorizontalOverflow(page);
});

test("long copy remains inside a 320px detail container independently of window width", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openV2Page(page, "/models?target=codex");
  const panel = page.getByRole("region", { name: "Codex 模型配置" });
  await expect(panel).toBeVisible();
  await panel.evaluate((element) => {
    element.style.width = "320px";
    for (const label of element.querySelectorAll("label")) {
      label.append(
        document.createTextNode("这是需要完整展示的说明文字".repeat(4)),
      );
    }
    const paragraph = element.querySelector(".fy-models-source-entry p");
    if (!paragraph) throw new Error("Missing source guidance");
    paragraph.textContent =
      "https://gateway.example/" + "LongProviderConfigurationName".repeat(8);
  });
  const form = panel.locator(".fy-models-form");
  await expect
    .poll(() =>
      form.evaluate(
        (element) =>
          getComputedStyle(element).gridTemplateColumns.split(" ").length,
      ),
    )
    .toBe(1);
  const escapes = await panel.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    return Array.from(
      element.querySelectorAll("label, p, button, input, select, textarea"),
    )
      .filter((child) => child.getClientRects().length > 0)
      .filter((child) => {
        const box = child.getBoundingClientRect();
        return box.left < rect.left - 1 || box.right > rect.right + 1;
      })
      .map((child) => child.outerHTML.slice(0, 180));
  });
  expect(escapes).toEqual([]);
  for (const width of [761, 759, 616]) {
    await page.setViewportSize({ width, height: 700 });
    await panel.evaluate((element) => {
      element.style.removeProperty("width");
    });
    await expectNoHorizontalOverflow(page);
    expect(
      await panel.evaluate(
        (element) => element.scrollWidth - element.clientWidth,
      ),
    ).toBeLessThanOrEqual(1);
  }
});

test("glass has blurred backing, readable fallback and fixed reachable actions", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  await openV2Page(page, "/auth");
  await page
    .getByRole("button", { name: "添加账号", exact: true })
    .first()
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  const overlay = page.locator(".fy-control-dialog-overlay");
  expect(
    await overlay.evaluate(
      (element) => getComputedStyle(element).backdropFilter,
    ),
  ).toContain("blur(");
  const glass = dialog.locator(".fy-frosted-surface");
  expect(
    await glass.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    ),
  ).toMatch(/0\.68/);
  // The fallback is intentionally independent of library/WebView SVG support.
  await page.emulateMedia({ reducedMotion: "reduce", forcedColors: "active" });
  expect(
    await glass.evaluate((element) => getComputedStyle(element).backdropFilter),
  ).toBe("none");
  await expect(dialog.getByRole("button", { name: "下一步" })).toBeInViewport();
  await page.emulateMedia({ forcedColors: "none" });
  const session = await page.context().newCDPSession(page);
  await session.send("Emulation.setEmulatedMedia", {
    features: [{ name: "prefers-reduced-transparency", value: "reduce" }],
  });
  expect(
    await glass.evaluate((element) => getComputedStyle(element).backdropFilter),
  ).toBe("none");
  const samples = await sampleTextContrast(page, ".fy-control-dialog");
  expect(samples.filter((sample) => sample.ratio < 4.5)).toEqual([]);
});
