import { expect, test } from "@playwright/test";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "./support/features";

test("prioritizes the initial module and never emits readiness from its loading shell", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  let release!: () => void;
  let intercepted = false;
  const moduleGate = new Promise<void>((resolve) => {
    release = resolve;
  });
  // Vite may append its cache-busting timestamp after a module changes. Match
  // the actual module pathname so the delay/failure injection still takes effect.
  await page.route(
    (url) => url.pathname === "/pages/agents/Page.tsx",
    async (route) => {
      intercepted = true;
      await moduleGate;
      await route.continue();
    },
  );
  try {
    await page.goto("/#/agents", { waitUntil: "domcontentloaded" });
    await expect.poll(() => intercepted).toBe(true);
    expect(
      (await featureFixtureCalls(page)).filter(
        (call) => call.command === "plugin:event|emit",
      ),
    ).toEqual([]);
    await expect(page.getByText("正在加载页面", { exact: true })).toHaveCount(
      0,
    );
    await expect(page.getByTestId("app-shell")).toHaveCount(0);
    release();
    await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
    await expect
      .poll(
        async () =>
          (await featureFixtureCalls(page)).filter(
            (call) =>
              call.command === "plugin:event|emit" &&
              call.payload.event === "frontend-deeplink-ready",
          ).length,
      )
      .toBe(1);
    expect(
      await page
        .locator(".fy-agent-directory-card img")
        .evaluateAll((images) =>
          images.every((image) => (image as HTMLImageElement).naturalWidth > 0),
        ),
    ).toBe(true);
  } finally {
    release();
  }
});

test("optional module preload failure does not break the initial page or become unhandled", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  let aborted = false;
  await page.route(
    (url) => url.pathname === "/pages/models/Page.tsx",
    async (route) => {
      aborted = true;
      await route.abort();
    },
  );
  await page.goto("/#/agents", { waitUntil: "domcontentloaded" });
  await expect.poll(() => aborted).toBe(true);
  await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
  await expect(page.getByRole("button", { name: "重新扫描" })).toBeEnabled();
  expect(errors).toEqual([]);
  await expect(
    page.getByRole("heading", { name: "页面暂时无法打开" }),
  ).toHaveCount(0);
});

test("a failed initial module shows a recoverable error rather than a permanent hidden window", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  const failedModule = (url: URL) => url.pathname === "/pages/agents/Page.tsx";
  await page.route(failedModule, (route) => route.abort());
  await page.goto("/#/agents", { waitUntil: "domcontentloaded" });
  await expect(
    page.getByRole("heading", { name: "页面暂时无法打开" }),
  ).toBeVisible();
  await expect(
    page.getByRole("alert").getByRole("button", { name: "重新加载界面" }),
  ).toBeVisible();
  await expect
    .poll(
      async () =>
        (await featureFixtureCalls(page)).filter(
          (call) =>
            call.command === "plugin:event|emit" &&
            call.payload.event === "frontend-deeplink-ready",
        ).length,
    )
    .toBe(1);
  await page.unroute(failedModule);
  await page.getByRole("button", { name: "重新加载界面" }).click();
  await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
});

test("prefetched Sessions stays unmounted until visited and preserves its search on return", async ({
  page,
}) => {
  await installRichTauriFeatureFixture(page);
  let prefetched = false;
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/pages/sessions/Page.tsx")
      prefetched = true;
  });
  await page.goto("/#/agents");
  await expect(page.locator(".fy-agent-directory-card")).toHaveCount(7);
  await expect.poll(() => prefetched).toBe(true);
  await expect(page.getByTestId("sessions-page")).toHaveCount(0);
  expect(
    (await featureFixtureCalls(page)).filter((call) =>
      [
        "list_sessions",
        "list_restore_attempts",
        "probe_local_provider",
      ].includes(call.command),
    ),
  ).toEqual([]);

  await page.locator('.fy-side-navigation a[href="#/sessions"]').click();
  const sessions = page.getByTestId("sessions-page");
  await expect(sessions).toBeVisible();
  await expect(sessions.getByRole("listitem")).toHaveCount(2);
  const search = sessions.getByRole("searchbox", { name: "搜索会话" });
  await search.fill("Alpha");
  await expect(sessions.getByRole("listitem")).toHaveCount(1);
  await expect(
    sessions.getByText("浏览器会话 Alpha", { exact: true }),
  ).toBeVisible();
  const originalSearch = await search.elementHandle();
  if (!originalSearch) throw new Error("Sessions search did not mount");

  await page.locator('.fy-side-navigation a[href="#/agents"]').click();
  await expect(page.getByTestId("agents-page")).toBeVisible();
  await expect(sessions).toHaveCount(1);
  await expect(sessions).toBeHidden();
  expect(
    await sessions.evaluate((node) => Boolean(node.closest("[hidden][inert]"))),
  ).toBe(true);

  // Observe the whole return rather than only checking the settled destination.
  await page.evaluate(() => {
    const state = { flashed: false };
    const observer = new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) {
          if (
            node instanceof Element &&
            (node.matches(".fy-feature-route-loading") ||
              node.querySelector(".fy-feature-route-loading"))
          )
            state.flashed = true;
        }
      }
    });
    observer.observe(document.body, { childList: true, subtree: true });
    Object.assign(window, { __sessionsReturn: { state, observer } });
  });
  await page.locator('.fy-side-navigation a[href="#/sessions"]').click();
  await expect(sessions).toBeVisible();
  await expect(search).toHaveValue("Alpha");
  await expect(sessions.getByRole("listitem")).toHaveCount(1);
  expect(
    await search.evaluate(
      (node, original) => node === original,
      originalSearch,
    ),
  ).toBe(true);
  expect(
    await page.evaluate(() => {
      const scope = window as typeof window & {
        __sessionsReturn?: {
          state: { flashed: boolean };
          observer: MutationObserver;
        };
      };
      const result = scope.__sessionsReturn;
      if (!result)
        throw new Error("Sessions return observer was not installed");
      result.observer.disconnect();
      delete scope.__sessionsReturn;
      return result.state.flashed;
    }),
  ).toBe(false);
  await originalSearch.dispose();
});
