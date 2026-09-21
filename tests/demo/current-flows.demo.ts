import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { expect, test, type Browser, type Page } from "@playwright/test";

import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "../browser/support";
import {
  featureFixtureCalls,
  installRichTauriFeatureFixture,
} from "../browser/support/features";

const root = fileURLToPath(new URL("../..", import.meta.url));
const origin = "http://127.0.0.1:4198";
const viewport = { width: 1440, height: 900 };
const demoKey = "demo-key-not-a-real-credential";
const demoBaseUrl = "https://workbuddy.example.test/v1";
const captureKind = process.env.FYAGENT_DEMO_CAPTURE_KIND ?? "debug";
const evidence =
  "Isolated browser fixture UI only; not native installation, credentials, rollback, Windows UAT, release or production evidence.";
const runId = `${new Date().toISOString().replace(/[:.]/gu, "-")}-${process.pid}`;
const output = path.join(root, "artifacts/current-demo", runId);
// Vite serves src as its root. A plain /artifacts URL returns the app HTML.
const publicPath = `/@fs${pathToFileURL(output).pathname}`;

type Cue = { start: number; end: number; text: string };
type Shot = {
  file: string;
  alt: string;
  route: string;
  clip: string;
  cue: number;
};
type Clip = {
  file: string;
  captions: string;
  title: string;
  elapsedMs: number;
  durationMs?: number;
  clockOffsetMs?: number;
  cues: Cue[];
};

function timestamp(ms: number) {
  return new Date(Math.max(0, Math.round(ms))).toISOString().slice(11, 23);
}

function escapeHtml(text: string) {
  return text.replace(
    /[&<>"']/gu,
    (char) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        char
      ]!,
  );
}

async function sha256(file: string) {
  return createHash("sha256")
    .update(await readFile(file))
    .digest("hex");
}

async function sourceIdentity() {
  const git = (args: string[]) =>
    execFileSync("git", args, { cwd: root, encoding: "utf8" });
  // Include untracked renderer dependencies and demo inputs in a shared tree.
  const files = [
    ...new Set(
      git([
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "-z",
        "--",
        "src",
        "config",
        "tests/browser/support.ts",
        "tests/browser/support",
        "tests/fixtures",
        "tests/demo",
        ".mise/tasks/frontend.toml",
        "package.json",
        "pnpm-lock.yaml",
        "scripts/version.mjs",
        "src-tauri/Cargo.toml",
        "src-tauri/tauri.conf.json",
        "src-tauri/src/commands/agent_catalog.rs",
      ])
        .split("\0")
        .filter(Boolean),
    ),
  ].sort();
  const sources: { file: string; sha256: string }[] = [];
  for (const file of files)
    sources.push({ file, sha256: await sha256(path.join(root, file)) });
  return {
    commit: git(["rev-parse", "HEAD"]).trim(),
    trackedSourceDirty:
      git(["status", "--porcelain", "--untracked-files=no"]).trim().length > 0,
    untrackedSourceFiles: git([
      "ls-files",
      "--others",
      "--exclude-standard",
      "--",
      "src",
      "config",
      "tests/demo",
    ])
      .trim()
      .split("\n")
      .filter(Boolean),
    digest: createHash("sha256").update(JSON.stringify(sources)).digest("hex"),
    sources,
  };
}

async function assertPrivateContentAbsent(page: Page) {
  await expectNoHorizontalOverflow(page);
  const privatePattern = /\/Users\/|[A-Z]:\\Users\\/u;
  await expect(page.locator("body")).not.toContainText(demoKey);
  await expect(page.locator("body")).not.toContainText(privatePattern);
  const visibleValues = await page
    .locator("input:not([type=password]),textarea")
    .evaluateAll((inputs) =>
      inputs.map((input) => (input as HTMLInputElement).value).join("\n"),
    );
  expect(visibleValues).not.toContain(demoKey);
  expect(visibleValues).not.toMatch(privatePattern);
}

async function fillWorkBuddy(page: Page) {
  const key = page.getByLabel("API Key", { exact: true });
  // Body text assertions alone do not detect a revealed password input.
  await expect(key).toHaveAttribute("type", "password");
  await page.getByLabel("服务地址").fill(demoBaseUrl);
  await key.fill(demoKey);
  await page.getByLabel("自定义模型 ID").fill("demo-model");
  await assertPrivateContentAbsent(page);
  await page.getByRole("button", { name: "保存并应用" }).click();
  await expect(page.getByRole("button", { name: "应用更改" })).toHaveCount(1);
  await expect(page.getByRole("button", { name: "应用更改" })).toBeVisible();
  await expect(page.getByRole("dialog", { name: "保存前确认" })).toHaveCount(0);
  const calls = await featureFixtureCalls(page);
  const previews = calls.filter(
    ({ command }) => command === "create_workbuddy_save_plan",
  );
  expect(previews).toHaveLength(1);
  expect(previews[0].payload).toMatchObject({
    request: { baseUrl: demoBaseUrl },
  });
  await expect(page.getByRole("region", { name: "将要更改" })).toContainText(
    demoBaseUrl,
  );
  expect(
    calls.filter(({ command }) => command === "apply_change_plan"),
  ).toHaveLength(0);
}

async function capture(
  browser: Browser,
  outcome: "failure" | "saved",
  shots: Shot[],
): Promise<Clip> {
  const clip: Clip = {
    file: outcome === "failure" ? "current-flows.webm" : "saved-result.webm",
    captions:
      outcome === "failure"
        ? "current-flows.zh-CN.vtt"
        : "saved-result.zh-CN.vtt",
    title:
      outcome === "failure"
        ? "首次使用、推荐与失败恢复状态"
        : "独立演示数据中的成功保存状态",
    elapsedMs: 0,
    cues: [],
  };
  const context = await browser.newContext({
    baseURL: origin,
    viewport,
    reducedMotion: "reduce",
    colorScheme: "light",
    recordVideo: { dir: path.join(output, "raw"), size: viewport },
  });
  const forbiddenRequests: string[] = [];
  await context.route("**/*", (route) => {
    const url = new URL(route.request().url());
    if (url.origin === origin) return route.continue();
    forbiddenRequests.push(url.origin); // Do not persist paths or query strings.
    return route.abort("blockedbyclient");
  });
  const page = await context.newPage();
  const started = Date.now();
  const video = page.video();
  const health = monitorPageHealth(page);
  const hold = async (caption: string, file: string) => {
    await assertPrivateContentAbsent(page);
    // Padding keeps captions inside a stable shot instead of page transitions.
    const start = Date.now() - started + 250;
    await page.screenshot({ path: path.join(output, file) });
    shots.push({
      file,
      alt: caption,
      route: new URL(page.url()).hash,
      clip: clip.file,
      cue: clip.cues.length + 1,
    });
    await page.waitForTimeout(3500); // Deliberate reading time, not readiness.
    clip.cues.push({ start, end: Date.now() - started - 250, text: caption });
  };
  try {
    await installRichTauriFeatureFixture(page, {
      firstUseGuideState: outcome === "failure" ? "pending" : "dismissed",
      workBuddySave: outcome,
    });
    if (outcome === "failure") {
      await openRendererPage(page, "/agents");
      await expect(
        page.getByRole("region", { name: "首次使用引导" }),
      ).toBeVisible();
      await hold(
        "先选择主要用途；也可以跳过引导，直接查看全部软件。",
        "01-purpose.png",
      );
      await page.getByRole("button", { name: "日常办公", exact: true }).click();
      await expect(
        page.getByRole("heading", { name: "WorkBuddy", exact: true }),
      ).toBeVisible();
      await hold(
        "办公用途推荐会说明适合的软件，之后可进入软件配置入口。",
        "02-recommendations.png",
      );
      await page
        .getByRole("region", { name: "首次使用引导" })
        .locator('[data-agent-id="workbuddy"]')
        .getByRole("button", { name: "开始配置" })
        .click();
      const existingConfig = page.getByRole("dialog", {
        name: "确认 WorkBuddy 配置方式",
      });
      await expect(existingConfig).toContainText("已配置 1 个模型");
      await hold(
        "已有配置时，可以保留或进入替换流程；这一步尚未修改配置。",
        "02-existing-config.png",
      );
      await existingConfig
        .getByRole("button", { name: "替换现有配置" })
        .click();
      await expect(page).toHaveURL(/#\/agents\?setup=workbuddy$/);
      await expect(
        page.getByRole("region", { name: "首次使用引导" }),
      ).toHaveCount(0);
      const directory = page.getByRole("region", { name: "AI 软件目录" });
      await expect(directory.getByRole("article")).toHaveCount(1);
      await expect(
        directory.getByRole("button", { name: "进行配置" }),
      ).toBeEnabled();
      await hold(
        "在选中软件的卡片确认安装状态，再进入模型配置。",
        "02-software-setup.png",
      );
      const calls = await featureFixtureCalls(page);
      expect(
        calls.some(({ command }) =>
          [
            "create_workbuddy_save_plan",
            "apply_change_plan",
            "fetch_workbuddy_models",
          ].includes(command),
        ),
      ).toBe(false);
      await directory.getByRole("button", { name: "进行配置" }).click();
      await expect(page).toHaveURL(/#\/models\?target=workbuddy&/);
    } else {
      await openRendererPage(page, "/models?target=workbuddy");
    }
    await fillWorkBuddy(page);
    if (outcome === "failure") {
      await hold(
        "填写服务地址和模型后，检查将修改的文件，再确认应用。",
        "03-save-preview.png",
      );
    }
    await page.getByRole("button", { name: "应用更改" }).click();
    await expect(page.locator("body")).toContainText(
      outcome === "failure"
        ? "保存失败，已恢复原配置"
        : "WorkBuddy 模型配置已保存",
    );
    await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
    const resultHeading = page.getByRole("heading", {
      name: outcome === "failure" ? "配置未应用" : "配置已应用",
      exact: true,
    });
    await resultHeading.evaluate((element) =>
      element.scrollIntoView({ block: "center", behavior: "instant" }),
    );
    await expect(resultHeading).toBeInViewport();
    await hold(
      outcome === "failure"
        ? "演示数据返回失败并已恢复状态；这不是原生文件回滚的实测证明。"
        : "另一组独立演示数据返回保存成功，凭据输入已清空；不代表真实账号已配置。",
      outcome === "failure"
        ? "04-recovered-failure.png"
        : "05-saved-result.png",
    );
    const calls = await featureFixtureCalls(page);
    const writes = calls.filter(
      ({ command }) => command === "apply_change_plan",
    );
    expect(writes).toHaveLength(1);
    // The confirmation sends only the opaque preview identity, never the key.
    expect(Object.keys(writes[0].payload ?? {}).sort()).toEqual([
      "planDigest",
      "planId",
    ]);
    expect(JSON.stringify(writes)).not.toContain(demoKey);
    expect(calls.some(({ command }) => command === "start_agent_action")).toBe(
      false,
    );
    expect(forbiddenRequests).toEqual([]);
    await expectHealthyPage(page, health);
  } finally {
    clip.elapsedMs = Date.now() - started;
    await context.close();
  }
  if (!video) throw new Error("The demonstration video was not recorded");
  await video.saveAs(path.join(output, clip.file));
  return clip;
}

function player(version: string, clips: Clip[], shots: Shot[]) {
  return `<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>FyAgent 使用演示</title><style>body{font:16px system-ui;max-width:1080px;margin:40px auto;padding:0 24px;background:#f6f7f9;color:#17202a}video,img{width:100%;border-radius:12px}figure,section{margin:32px 0}p,figcaption{line-height:1.7}</style><h1>FyAgent 使用演示</h1><p>版本 ${escapeHtml(version)} · ${captureKind === "debug" ? "调试采集" : "候选采集"} · 隔离的浏览器演示数据，仅说明当前界面，不代表真实账号、原生回滚或安装验收。</p>${clips.map((clip) => `<section><h2>${escapeHtml(clip.title)}</h2><video aria-label="${escapeHtml(clip.title)}" controls muted preload="metadata"><source src="${clip.file}" type="video/webm"><track kind="subtitles" src="${clip.captions}" srclang="zh-CN" label="中文字幕" default></video></section>`).join("")}${shots.map((shot) => `<figure><img src="${shot.file}" alt="${escapeHtml(shot.alt)}"><figcaption>${escapeHtml(shot.alt)}</figcaption></figure>`).join("")}<p><a href="manifest.json">素材、源码版本和校验值</a></p></html>`;
}

test("capture and play back current fixture flows with bound captions", async ({
  browser,
}) => {
  expect(["debug", "candidate"]).toContain(captureKind);
  const before = await sourceIdentity();
  const version = execFileSync(
    process.execPath,
    [path.join(root, "scripts/version.mjs"), "get"],
    { cwd: root, encoding: "utf8" },
  ).trim();
  await mkdir(output, { recursive: true });
  const shots: Shot[] = [];
  const clips = [
    await capture(browser, "failure", shots),
    await capture(browser, "saved", shots),
  ];
  // Probe real encoded duration before binding and validating subtitle timing.
  const context = await browser.newContext({ baseURL: origin });
  const playback = await context.newPage();
  try {
    for (const clip of clips) {
      const response = await playback.goto(`${publicPath}/${clip.file}`);
      expect(response?.headers()["content-type"]).toContain("video/webm");
      const video = playback.locator("video");
      await expect
        .poll(() =>
          video.evaluate(
            (element: HTMLVideoElement) =>
              Number.isFinite(element.duration) && element.duration > 0,
          ),
        )
        .toBe(true);
      clip.durationMs = await video.evaluate(
        (element: HTMLVideoElement) => element.duration * 1000,
      );
      clip.clockOffsetMs = clip.durationMs - clip.elapsedMs;
      expect(
        Math.abs(clip.clockOffsetMs),
        "Unexpected recording clock drift",
      ).toBeLessThan(1500);
      clip.cues = clip.cues.map((cue) => ({
        ...cue,
        start: Math.max(0, cue.start + clip.clockOffsetMs!),
        end: Math.min(clip.durationMs!, cue.end + clip.clockOffsetMs!),
      }));
      for (const cue of clip.cues) expect(cue.end).toBeGreaterThan(cue.start);
      await writeFile(
        path.join(output, clip.captions),
        "WEBVTT\n\n" +
          clip.cues
            .map(
              (cue, index) =>
                `${index + 1}\n${timestamp(cue.start)} --> ${timestamp(cue.end)}\n${cue.text}\n`,
            )
            .join("\n"),
      );
    }
    await writeFile(
      path.join(output, "index.html"),
      player(version, clips, shots),
    );
    await playback.goto(`${publicPath}/index.html`);
    for (const [index, clip] of clips.entries()) {
      const video = playback.locator("video").nth(index);
      await expect
        .poll(() =>
          video.evaluate(
            (element: HTMLVideoElement) =>
              element.textTracks[0]?.cues?.length ?? 0,
          ),
        )
        .toBe(clip.cues.length);
      for (const cue of clip.cues) {
        await video.evaluate(
          (element: HTMLVideoElement, time) => {
            element.currentTime = time;
          },
          (cue.start + cue.end) / 2000,
        );
        await expect
          .poll(() =>
            video.evaluate(
              (element: HTMLVideoElement) =>
                (element.textTracks[0]?.activeCues?.[0] as VTTCue | undefined)
                  ?.text ?? "",
            ),
          )
          .toBe(cue.text);
      }
      await video.evaluate(async (element: HTMLVideoElement) => {
        element.currentTime = 0;
        element.muted = true;
        await element.play();
      });
      await expect
        .poll(
          () => video.evaluate((element: HTMLVideoElement) => element.ended),
          { timeout: clip.durationMs! + 5000 },
        )
        .toBe(true);
    }
    await expect
      .poll(() =>
        playback
          .locator("img")
          .evaluateAll((images) =>
            images.every(
              (image) => (image as HTMLImageElement).naturalWidth > 0,
            ),
          ),
      )
      .toBe(true);
  } finally {
    await context.close();
  }
  const after = await sourceIdentity();
  const sourceStable =
    before.commit === after.commit && before.digest === after.digest;
  if (captureKind === "candidate")
    expect(
      sourceStable,
      "Renderer or fixture sources changed during capture",
    ).toBe(true);
  const assets = [];
  for (const file of [
    ...shots.map(({ file }) => file),
    ...clips.flatMap(({ file, captions }) => [file, captions]),
    "index.html",
  ]) {
    assets.push({ file, sha256: await sha256(path.join(output, file)) });
  }
  await writeFile(
    path.join(output, "manifest.json"),
    JSON.stringify(
      {
        schemaVersion: 2,
        captureKind,
        version,
        versionSource: "src-tauri/Cargo.toml [workspace.package].version",
        runId,
        capturedAt: new Date().toISOString(),
        sourceCommit: before.commit,
        sourceStable,
        trackedSourceDirty:
          before.trackedSourceDirty || after.trackedSourceDirty,
        untrackedSourceFiles: before.untrackedSourceFiles,
        sourceDigestBefore: before.digest,
        sourceDigestAfter: after.digest,
        evidence,
        privacy:
          "Only synthetic credentials and fixture state; passwords remain masked; external HTTP requests blocked; no account or installer access.",
        captionClock:
          "End-aligned to encoded media duration with 250 ms padding inside each stable shot; playback checked every cue and both full videos.",
        playbackVerified: true,
        viewport,
        clips,
        shots,
        sources: before.sources,
        assets,
      },
      null,
      2,
    ) + "\n",
  );
  console.info(
    `Fixture demo artifacts (${captureKind}, sourceStable=${sourceStable}): ${output}`,
  );
});
