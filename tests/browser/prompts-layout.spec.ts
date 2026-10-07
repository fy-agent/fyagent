import {
  expect,
  test,
  type Locator,
  type Page,
  type TestInfo,
} from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

const shortName = "审查规范";
const longName = "长名称提示词：".repeat(18);
const description = "多行描述，保留每一行的说明。\n".repeat(40);
const content = "正文：逐行检查并保留未保存的草稿。\n".repeat(180);
const liveContent = "live：当前原生文件的只读样本。\n".repeat(180);
const viewports = [
  { width: 1234, height: 732 },
  { width: 900, height: 600 },
  { width: 1440, height: 900 },
];

async function installPromptFixture(
  page: Page,
  failure: "none" | "write-and-live-read" | "library-read" = "none",
) {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(
    ({ shortName, longName, description, content, liveContent, failure }) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const rows = {
        short: {
          id: "short",
          name: shortName,
          description: "简短说明",
          content: "短正文",
          enabled: false,
        },
        long: {
          id: "long",
          name: longName,
          description,
          content,
          enabled: false,
        },
      };
      let libraryReadFailed = failure === "library-read";
      document.addEventListener(
        "click",
        (event) => {
          if (
            event.target instanceof Element &&
            event.target.closest("button")?.textContent?.trim() === "重试"
          ) {
            libraryReadFailed = false;
          }
        },
        true,
      );
      window.__TAURI_INTERNALS__.invoke = async (command, payload) => {
        if (command === "get_prompts") {
          if (payload?.app === "claude" && libraryReadFailed) {
            throw new Error("I10 fixture library read failed");
          }
          return rows;
        }
        if (command === "get_current_prompt_file_content") {
          if (failure === "write-and-live-read")
            throw new Error("I10 fixture live read failed");
          return liveContent;
        }
        if (command === "upsert_prompt") {
          window.__FYAGENT_FEATURE_FIXTURE__.calls.push({
            command,
            payload: payload ?? {},
          });
          throw new Error(
            failure === "write-and-live-read"
              ? "I10 fixture library save failed"
              : "Unexpected save in I10 layout fixture",
          );
        }
        return invoke(command, payload);
      };
    },
    { shortName, longName, description, content, liveContent, failure },
  );
}

// Read-only geometry includes clipping ancestors, not just the window.
async function visibleBox(area: Locator) {
  return area.evaluate((node) => {
    const box = node.getBoundingClientRect();
    let left = Math.max(0, box.left);
    let right = Math.min(innerWidth, box.right);
    let top = Math.max(0, box.top);
    let bottom = Math.min(innerHeight, box.bottom);
    for (
      let parent = node.parentElement;
      parent;
      parent = parent.parentElement
    ) {
      const style = getComputedStyle(parent);
      const bounds = parent.getBoundingClientRect();
      if (/auto|scroll|hidden|clip/.test(style.overflowY)) {
        top = Math.max(top, bounds.top);
        bottom = Math.min(bottom, bounds.bottom);
      }
      if (/auto|scroll|hidden|clip/.test(style.overflowX)) {
        left = Math.max(left, bounds.left);
        right = Math.min(right, bounds.right);
      }
    }
    return { left, right, top, bottom, height: box.height };
  });
}

async function wheelPoint(owner: Locator, direction: number) {
  const box = await visibleBox(owner);
  if (box.right - box.left <= 10 || box.bottom - box.top <= 10) return null;
  return owner.evaluate(
    (node, { box, direction }) => {
      const inset = Math.min(6, (box.right - box.left) / 4);
      const xs = [
        box.left + inset,
        box.right - inset,
        (box.left + box.right) / 2,
      ];
      const ys = [0.5, 0.1, 0.9].map(
        (fraction) => box.top + (box.bottom - box.top) * fraction,
      );
      for (const x of xs) {
        for (const y of ys) {
          const hit = document.elementFromPoint(x, y);
          if (hit === null || (hit !== node && !node.contains(hit))) continue;
          let receiver: Element | null = hit;
          while (receiver) {
            const style = getComputedStyle(receiver);
            if (/auto|scroll/.test(style.overflowY)) {
              const hasRange =
                direction < 0
                  ? receiver.scrollTop > 1
                  : receiver.scrollTop <
                    receiver.scrollHeight - receiver.clientHeight - 1;
              // Empty or exhausted contain/none containers still block chaining.
              // Only auto at this direction's boundary can pass to its parent.
              if (hasRange || /contain|none/.test(style.overscrollBehaviorY))
                break;
            }
            receiver = receiver.parentElement;
          }
          if (receiver === node) return { x, y };
        }
      }
      return null;
    },
    { box, direction },
  );
}

async function wheelInside(page: Page, owner: Locator, delta: number) {
  const direction = Math.sign(delta);
  const point = await wheelPoint(owner, direction);
  expect(
    point,
    "A visible hit must belong to the direction's actual wheel receiver",
  ).not.toBeNull();
  expect(
    await owner.evaluate(
      (node, { point, direction }) => {
        let receiver = document.elementFromPoint(point!.x, point!.y);
        while (receiver) {
          const style = getComputedStyle(receiver);
          if (/auto|scroll/.test(style.overflowY)) {
            const hasRange =
              direction < 0
                ? receiver.scrollTop > 1
                : receiver.scrollTop <
                  receiver.scrollHeight - receiver.clientHeight - 1;
            if (hasRange || /contain|none/.test(style.overscrollBehaviorY))
              break;
          }
          receiver = receiver.parentElement;
        }
        return receiver === node;
      },
      { point, direction },
    ),
    `Wheel receiver must equal its owner: ${JSON.stringify({ point, direction })}`,
  ).toBe(true);
  await page.mouse.move(point!.x, point!.y);
  await page.mouse.wheel(0, delta);
}

// Reveal controls by wheeling actual ancestors. Never write scrollTop or use
// scrollIntoView/action auto-scroll as physical-scroll evidence.

async function revealByWheel(page: Page, target: Locator) {
  for (let step = 0; step < 12; step++) {
    const visible = await visibleBox(target);
    if (
      visible.right - visible.left > 20 &&
      visible.bottom - visible.top >= Math.min(36, visible.height)
    )
      return;
    const targetTop = await target.evaluate(
      (node) => node.getBoundingClientRect().top,
    );
    const direction = targetTop < visible.top - 1 ? -1 : 1;
    const ancestors = target.locator("xpath=ancestor::*");
    const candidates = await ancestors.evaluateAll(
      (nodes, direction) =>
        nodes
          .map((node, index) => {
            const element = node as HTMLElement;
            let depth = 0;
            for (
              let parent = element.parentElement;
              parent;
              parent = parent.parentElement
            )
              depth++;
            return {
              index,
              depth,
              eligible:
                /auto|scroll/.test(getComputedStyle(element).overflowY) &&
                element.scrollHeight > element.clientHeight + 2 &&
                (direction < 0
                  ? element.scrollTop > 1
                  : element.scrollTop <
                    element.scrollHeight - element.clientHeight - 1),
            };
          })
          .filter((candidate) => candidate.eligible)
          .sort((first, second) => second.depth - first.depth),
      direction,
    );
    let owner: Locator | null = null;
    for (const candidate of candidates) {
      const ancestor = ancestors.nth(candidate.index);
      if (await wheelPoint(ancestor, direction)) {
        owner = ancestor;
        break;
      }
    }
    expect(
      owner,
      "A reachable actual wheel receiver must reveal the control",
    ).not.toBeNull();
    const before = await owner!.evaluate((node) => node.scrollTop);
    const distance = Math.min(
      260,
      Math.max(40, visible.top - visible.bottom + 36),
    );
    await wheelInside(page, owner!, direction * distance);
    const wheelDiagnostic = await owner!.evaluate((node) => ({
      owner: node.className,
      top: node.scrollTop,
      height: node.clientHeight,
      scrollHeight: node.scrollHeight,
      bounds: node.getBoundingClientRect().toJSON(),
      children: Array.from(node.children).map((child) => ({
        name: child.className,
        bounds: child.getBoundingClientRect().toJSON(),
        top: child.scrollTop,
        height: child.clientHeight,
        scrollHeight: child.scrollHeight,
      })),
    }));
    await expect
      .poll(() => owner!.evaluate((node) => node.scrollTop), {
        message: JSON.stringify({
          before,
          direction,
          distance,
          visible,
          wheelDiagnostic,
        }),
      })
      .not.toBe(before);
  }
  const visible = await visibleBox(target);
  expect(
    visible.bottom - visible.top,
    "Control is physically reachable",
  ).toBeGreaterThanOrEqual(Math.min(36, visible.height));
}

const surroundingsSelector =
  '[data-testid="top-bar"], [data-testid="content-viewport"], .fy-prompts-app-rail, .fy-prompts-library-pane, .fy-prompts-editor-pane, .fy-prompts-editor-head, .fy-prompts-editor-identity, .fy-prompts-live summary, .fy-prompts-editor-content, .fy-prompts-editor-description, .fy-prompts-live-content';

async function surroundings(page: Page) {
  return page.locator(surroundingsSelector).evaluateAll((nodes) =>
    nodes.map((node) => {
      const box = node.getBoundingClientRect();
      return {
        x: box.x,
        y: box.y,
        width: box.width,
        height: box.height,
        scrollTop: node.scrollTop,
      };
    }),
  );
}

async function expectIndependentWheel(page: Page, owner: Locator) {
  await revealByWheel(page, owner);
  expect(
    await owner.evaluate((node) => node.scrollHeight - node.clientHeight),
  ).toBeGreaterThan(100);
  await wheelInside(page, owner, -100_000);
  await expect
    .poll(() => owner.evaluate((node) => node.scrollTop))
    .toBeLessThan(2);
  const before = await surroundings(page);
  const ownerIndex = await owner.evaluate(
    (ownerNode, selector) =>
      Array.from(document.querySelectorAll(selector)).indexOf(ownerNode),
    surroundingsSelector,
  );
  expect(ownerIndex).toBeGreaterThanOrEqual(0);
  await wheelInside(page, owner, 320);
  await expect
    .poll(() => owner.evaluate((node) => node.scrollTop))
    .toBeGreaterThan(10);
  const after = await surroundings(page);
  expect(after).toHaveLength(before.length);
  for (const [index, previous] of before.entries()) {
    for (const dimension of ["x", "y", "width", "height"] as const) {
      expect(
        Math.abs(after[index][dimension] - previous[dimension]),
        `Region ${index} ${dimension}`,
      ).toBeLessThanOrEqual(1);
    }
    if (index !== ownerIndex) {
      expect(
        Math.abs(after[index].scrollTop - previous.scrollTop),
        `Other region ${index} must not scroll`,
      ).toBeLessThanOrEqual(1);
    }
  }
  await wheelInside(page, owner, -100_000);
  await expect
    .poll(() => owner.evaluate((node) => node.scrollTop))
    .toBeLessThan(2);
}

async function expectReachableFocus(control: Locator) {
  await expect(control).toBeFocused();
  await expect(control).toBeEnabled();
  await expect
    .poll(
      async () => {
        const box = await visibleBox(control);
        return (
          box.right - box.left > 20 &&
          box.bottom - box.top >= Math.min(36, box.height)
        );
      },
      { message: "Tab must reveal an interactive part of its focused control" },
    )
    .toBe(true);
  const box = await visibleBox(control);
  const point = {
    x: (box.left + box.right) / 2,
    y: (box.top + box.bottom) / 2,
  };
  expect(
    await control.evaluate((node, point) => {
      const hit = document.elementFromPoint(point.x, point.y);
      return hit === node || (hit !== null && node.contains(hit));
    }, point),
    "Focused control must be unobscured at its visible center",
  ).toBe(true);
}
async function capture(page: Page, info: TestInfo, state: string) {
  const viewport = page.viewportSize()!;
  const name = `prompts-${viewport.width}x${viewport.height}-${state}.png`;
  const path = info.outputPath(name);
  await page.screenshot({ path });
  await info.attach(name, { path, contentType: "image/png" });
  await info.attach(`${state}-viewport-binding`, {
    body: JSON.stringify({
      evidenceKind: "runtime_screenshot",
      scope: "browser Tauri fixture; native acceptance remains open",
      project: info.project.name,
      viewport,
      screenshot: path,
    }),
    contentType: "application/json",
  });
}

for (const viewport of viewports) {
  test(`I10 prompts layout and independent wheel at ${viewport.width}x${viewport.height}`, async ({
    page,
  }, info) => {
    await page.setViewportSize(viewport);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await installPromptFixture(page);
    const health = monitorPageHealth(page);
    await openRendererPage(page, "/prompts?target=claude");
    const scope = page.getByTestId("prompts-page");
    const pane = scope.getByRole("region", { name: "提示词详情", exact: true });
    const body = pane.getByRole("textbox", { name: "内容", exact: true });
    const name = pane.getByRole("textbox", { name: "名称", exact: true });
    const desc = pane.getByRole("textbox", { name: "描述", exact: true });
    const live = pane.getByRole("textbox", {
      name: "当前使用的内容",
      exact: true,
    });
    const summary = pane.locator("summary");
    await expect(name).toHaveValue(shortName);
    await expect(body).toHaveValue("短正文");
    await expect(pane.locator("details")).not.toHaveAttribute("open", "");
    await expect(live).not.toBeVisible();
    await capture(page, info, "short-collapsed");
    await scope
      .getByRole("region", { name: "提示词列表", exact: true })
      .getByRole("button", { name: new RegExp(longName) })
      .click();
    await expect(name).toHaveValue(longName);
    await expect(desc).toHaveValue(description);
    await expect(body).toHaveValue(content);
    const title = pane.getByRole("heading", { name: longName, exact: true });
    await expect(title).toHaveAttribute("title", longName);
    const titleGeometry = await title.evaluate((node) => ({
      height: node.getBoundingClientRect().height,
      lineHeight: Number.parseFloat(getComputedStyle(node).lineHeight),
    }));
    expect(titleGeometry.lineHeight).toBeGreaterThan(0);
    expect(titleGeometry.height).toBeLessThanOrEqual(
      titleGeometry.lineHeight * 2 + 1,
    );
    await expectIndependentWheel(page, body);
    await expectIndependentWheel(page, desc);
    await revealByWheel(page, summary);
    await summary.click();
    await expect(pane.locator("details")).toHaveAttribute("open", "");
    await expect(live).toHaveValue(liveContent);
    await expect(live).toHaveAttribute("readonly", "");
    await expectIndependentWheel(page, live);
    await capture(page, info, "long-live-expanded");

    // Focus is Tab-order setup, not physical scrolling evidence.
    await body.focus();
    await page.keyboard.press("Shift+Tab");
    await expect(
      pane.getByRole("button", { name: "删除", exact: true }),
    ).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    await expect(
      pane.getByRole("button", { name: "保存", exact: true }),
    ).toBeFocused();
    for (const control of [
      pane.getByRole("button", { name: "删除", exact: true }),
      body,
      name,
      desc,
      summary,
      live,
    ]) {
      await page.keyboard.press("Tab");
      await expectReachableFocus(control);
    }
    await page.keyboard.press("Shift+Tab");
    await expectReachableFocus(summary);
    await page.keyboard.press("Enter");
    await expect(live).not.toBeVisible();
    await expect(name).toHaveValue(longName);
    await expect(desc).toHaveValue(description);
    await expect(body).toHaveValue(content);
    await expectNoHorizontalOverflow(page);
    await expectHealthyPage(page, health);
    expect(
      await page.evaluate(() =>
        window.__FYAGENT_FEATURE_FIXTURE__.calls.filter((call) =>
          /^(upsert_prompt|enable_prompt|delete_prompt|import_prompt_from_file)$/.test(
            call.command,
          ),
        ),
      ),
    ).toEqual([]);
    await capture(page, info, "long-live-collapsed");
  });
}

test("I10 minimum window keeps read/write errors visible and failed-save draft editable", async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 900, height: 600 });
  await installPromptFixture(page, "write-and-live-read");
  await openRendererPage(page, "/prompts?target=claude");
  const scope = page.getByTestId("prompts-page");
  const body = scope.getByRole("textbox", { name: "内容", exact: true });
  const draft = `${content}未保存的末行`;
  await body.fill(draft);
  await scope.locator(".fy-prompts-live summary").click();
  const readError = scope.getByText(
    "暂时无法读取当前使用的内容：请稍后重试。",
    { exact: true },
  );
  await revealByWheel(page, readError);
  await expect(readError).toBeInViewport();
  await expect(scope.getByText(/I10 fixture live read failed/)).toHaveCount(0);
  await capture(page, info, "live-read-error");
  await scope.getByRole("button", { name: "保存", exact: true }).click();
  const writeError = scope.getByText("提示词已保存失败：请稍后重试。", {
    exact: true,
  });
  await expect(writeError).toBeVisible();
  await expect(writeError).toBeInViewport();
  await expect(scope.getByText(/I10 fixture library save failed/)).toHaveCount(
    0,
  );
  await expect(body).toHaveValue(draft);
  await expect(
    scope.getByRole("textbox", { name: "名称", exact: true }),
  ).toHaveValue(shortName);
  await capture(page, info, "save-error-draft-retained");
  await body.focus();
  await page.keyboard.press("Control+End");
  await page.keyboard.type("，继续编辑");
  await expect(body).toHaveValue(`${draft}，继续编辑`);
  await expect(writeError).toHaveCount(0);
  const calls = await page.evaluate(() =>
    window.__FYAGENT_FEATURE_FIXTURE__.calls.filter(
      (call) => call.command === "upsert_prompt",
    ),
  );
  expect(calls).toHaveLength(1);
  expect(calls[0].payload).toMatchObject({
    app: "claude",
    id: "short",
    prompt: { id: "short", name: shortName, content: draft, enabled: false },
  });
  await expectNoHorizontalOverflow(page);
});

test("I10 minimum window exposes library read error and explicit retry", async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 900, height: 600 });
  await installPromptFixture(page, "library-read");
  await openRendererPage(page, "/prompts?target=claude");
  const scope = page.getByTestId("prompts-page");
  await expect(
    scope.getByText("无法加载 Claude Code 提示词", { exact: true }),
  ).toBeInViewport();
  await expect(
    scope.getByText("请稍后重试。", { exact: true }),
  ).toBeInViewport();
  await expect(scope.getByText(/I10 fixture library read failed/)).toHaveCount(
    0,
  );
  const retry = scope.getByRole("button", { name: "重试", exact: true });
  await expect(retry).toBeInViewport();
  await capture(page, info, "library-read-error");
  await retry.click();
  await expect(
    scope.getByRole("textbox", { name: "内容", exact: true }),
  ).toHaveValue("短正文");
  await expect(
    scope.getByText("无法加载 Claude Code 提示词", { exact: true }),
  ).toHaveCount(0);
});
