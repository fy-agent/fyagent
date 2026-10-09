import { mkdir } from "node:fs/promises";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { installRichTauriFeatureFixture } from "./support/features";
import {
  expectHealthyPage,
  expectNoHorizontalOverflow,
  monitorPageHealth,
  openRendererPage,
} from "./support";

type Scenario = "skills" | "mcp" | "linked" | "sessions";
interface SprintEvent {
  command: string;
  payload: Record<string, unknown>;
  observed?: unknown;
}
interface SprintSnapshot {
  skillIds: string[];
  skillApps: Record<string, Record<string, boolean>>;
  unmanagedDirectories: string[];
  mcpApps: Record<string, Record<string, boolean>>;
  events: SprintEvent[];
  attemptStage: string;
  hasAttestation: boolean;
}
declare global {
  interface Window {
    __FYAGENT_SPRINT_FIXTURE__: { snapshot(): SprintSnapshot };
  }
}

// Controlled Tauri-boundary runtime only: no filesystem, CLI, accounts or network.
async function installSprintFixture(page: Page, scenario: Scenario) {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(
    ({ scenario }) => {
      const targets = [
        "qoderwork",
        "trae-work",
        "workbuddy",
        "grokbuild",
        "codex",
        "claude",
        "opencode",
      ];
      const apps = (enabled: string[] = []): Record<string, boolean> =>
        Object.fromEntries(targets.map((id) => [id, enabled.includes(id)]));
      const unmanaged = Array.from({ length: 70 }, (_, index) => ({
        directory: `sprint-directory-${index}`,
        name: index === 69 ? "unique-import-skill" : `sprint-skill-${index}`,
        description: "Controlled observation only",
        path: `/synthetic/sprint/source-${index}`,
        foundIn: ["claude"],
        readOnly: false,
      }));
      const skills: Array<{
        id: string;
        directory: string;
        name: string;
        description: string;
        apps: Record<string, boolean>;
        installedAt: number;
        updatedAt: number;
        contentHash: string;
        readOnly: boolean;
        readOnlyTargets: string[];
      }> =
        scenario === "linked"
          ? [
              {
                id: "local:sprint-linked",
                directory: "sprint-linked",
                name: "Observed Linked Skill",
                description:
                  "Synthetic linked-source and linked-target DTO; no real link created",
                apps: apps(["claude"]),
                installedAt: 0,
                updatedAt: 1,
                contentHash: "synthetic-linked-content",
                readOnly: true,
                readOnlyTargets: ["claude"],
              },
            ]
          : [];
      const servers: Record<
        string,
        {
          id: string;
          name: string;
          description: string;
          server: { type: string; command: string };
          apps: Record<string, boolean>;
        }
      > = Object.fromEntries(
        ["success", "throw", "mismatch"].map((id) => [
          id,
          {
            id,
            name: `Sprint MCP ${id}`,
            description: "Controlled MCP assignment",
            server: { type: "stdio", command: "synthetic-never-executed" },
            apps: apps(),
          },
        ]),
      );
      const mcpWriteCounts: Record<string, number> = {};
      const events: SprintEvent[] = [];
      const attempt = {
        attemptId: "22222222-2222-4222-8222-222222222222",
        requestId: "33333333-3333-4333-8333-333333333333",
        snapshotId: `fys1:${"b".repeat(64)}`,
        requestKind: "defaultImport",
        idempotencySlot: `fyslot1:${"c".repeat(64)}`,
        contentDigest: `fyc1:${"a".repeat(64)}`,
        origin: {
          originId: "11111111-1111-4111-8111-111111111111",
          providerId: "codex",
          sessionId: "sprint-source",
          cliVersion: "0.154.0",
        },
        targetProviderId: "codex",
        targetStoreId: "synthetic-sprint-store",
        targetNativeNonce: "synthetic-sprint-nonce",
        targetNativeId: "synthetic-sprint-native",
        targetWorkspace: "/synthetic/sprint/workspace",
        stage: "nextTurnRequestVerified",
        attemptCount: 1,
        createdAt: 1795478400000,
        updatedAt: 1795478401000,
        userAttestation: null as null | {
          attestedAt: number;
          claimedStage: string;
          note?: string;
        },
      };
      const snapshot = (): SprintSnapshot =>
        structuredClone({
          skillIds: skills.map((row) => row.id),
          skillApps: Object.fromEntries(
            skills.map((row) => [row.id, row.apps]),
          ),
          unmanagedDirectories: unmanaged
            .filter(
              (row) =>
                !skills.some((skill) => skill.directory === row.directory),
            )
            .map((row) => row.directory),
          mcpApps: Object.fromEntries(
            Object.entries(servers).map(([id, row]) => [id, row.apps]),
          ),
          events,
          attemptStage: attempt.stage,
          hasAttestation: attempt.userAttestation !== null,
        });
      window.__FYAGENT_SPRINT_FIXTURE__ = { snapshot };
      const delegate = window.__TAURI_INTERNALS__.invoke;
      const observed = (event: SprintEvent, value: unknown) => {
        event.observed = structuredClone(value);
        return structuredClone(value);
      };
      window.__TAURI_INTERNALS__.invoke = async (command, payload = {}) => {
        const handled = [
          "get_installed_skills",
          "scan_unmanaged_skills",
          "import_skills_from_apps",
          "toggle_skill_app",
          "get_mcp_servers",
          "toggle_mcp_app",
        ];
        if (scenario === "sessions")
          handled.push(
            "list_restore_attempts",
            "record_user_attestation",
            "get_release_capability_matrix",
          );
        if (!handled.includes(command)) return delegate(command, payload);
        const event: SprintEvent = {
          command,
          payload: structuredClone(payload),
        };
        events.push(event);
        window.__FYAGENT_FEATURE_FIXTURE__.calls.push({ command, payload });
        switch (command) {
          case "get_installed_skills":
            return observed(event, skills);
          case "scan_unmanaged_skills":
            return observed(
              event,
              unmanaged.filter(
                (row) =>
                  !skills.some((skill) => skill.directory === row.directory),
              ),
            );
          case "import_skills_from_apps": {
            const selections = payload.imports as Array<{
              directory: string;
              apps: Record<string, boolean>;
            }>;
            const imported = selections.map((selection) => {
              const source = unmanaged.find(
                (row) => row.directory === selection.directory,
              );
              if (
                !source ||
                skills.some((row) => row.directory === selection.directory)
              )
                throw new Error(
                  "Unexpected duplicate or unknown synthetic import",
                );
              const row = {
                id: `local:${source.directory}`,
                directory: source.directory,
                name: source.name,
                description: source.description,
                apps: { ...selection.apps },
                installedAt: 0,
                updatedAt: 1,
                contentHash: "synthetic-imported-content",
                readOnly: false,
                readOnlyTargets: [] as string[],
              };
              skills.push(row);
              return row;
            });
            return observed(event, imported);
          }
          case "toggle_skill_app": {
            const row = skills.find((skill) => skill.id === payload.id);
            const target = String(payload.app);
            if (
              !row ||
              !targets.includes(target) ||
              row.readOnlyTargets.includes(target)
            )
              throw new Error("Synthetic target guard refused mutation");
            row.apps[target] = Boolean(payload.enabled);
            return true;
          }
          case "get_mcp_servers":
            return observed(event, servers);
          case "toggle_mcp_app": {
            const id = String(payload.serverId);
            const row = servers[id];
            if (!row || !targets.includes(String(payload.app)))
              throw new Error("Unknown synthetic MCP target");
            mcpWriteCounts[id] = (mcpWriteCounts[id] ?? 0) + 1;
            if (id === "throw" && mcpWriteCounts[id] === 1)
              throw new Error("Synthetic target write failed");
            if (id === "mismatch" && mcpWriteCounts[id] === 1) return undefined;
            row.apps[String(payload.app)] = Boolean(payload.enabled);
            return undefined;
          }
          case "list_restore_attempts":
            return observed(event, [attempt]);
          case "get_release_capability_matrix":
            return [];
          case "record_user_attestation":
            if (payload.attemptId !== attempt.attemptId)
              throw new Error("Unknown synthetic receipt");
            attempt.userAttestation = {
              attestedAt: 1795478402000,
              claimedStage: String(payload.claimedStage),
              ...(typeof payload.note === "string"
                ? { note: payload.note }
                : {}),
            };
            return observed(event, attempt);
          default:
            throw new Error(`Unimplemented synthetic command ${command}`);
        }
      };
    },
    { scenario },
  );
}

async function snapshot(page: Page) {
  return page.evaluate(() => window.__FYAGENT_SPRINT_FIXTURE__.snapshot());
}
const writes = (state: SprintSnapshot, command: string) =>
  state.events.filter((event) => event.command === command);

async function captureSprintEvidence(page: Page, filename: string) {
  // Ordinary CI owns artifacts per test/project; controlled exports remain opt-in.
  const info = test.info();
  const exportRoot = process.env.FYAGENT_SPRINT_EVIDENCE_DIR;
  const destination = exportRoot
    ? path.join(exportRoot, info.project.name, filename)
    : info.outputPath(filename);
  await mkdir(path.dirname(destination), { recursive: true });
  await page.screenshot({ path: destination, fullPage: false });
  await info.attach(filename, { path: destination, contentType: "image/png" });
}

test.use({ viewport: { width: 900, height: 600 }, trace: "off" });

test("Skills imports one of seventy only after preview confirmation, then verifies its bulk assignment", async ({
  page,
}) => {
  await installSprintFixture(page, "skills");
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/skills");
  const initial = await snapshot(page);
  expect(initial.unmanagedDirectories).toHaveLength(70);
  await page.getByRole("button", { name: "更多", exact: true }).click();
  await page
    .getByRole("button", { name: "导入本地 Skill", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "导入本地 Skills",
    exact: true,
  });
  await expect(
    dialog.getByRole("checkbox", {
      name: "选择 unique-import-skill",
      exact: true,
    }),
  ).toBeAttached();
  await expect(
    dialog.getByRole("button", { name: "预览导入 · 0", exact: true }),
  ).toBeDisabled();
  const search = dialog.getByRole("searchbox", {
    name: "筛选本地 Skills",
    exact: true,
  });
  await search.fill("unique-import");
  await dialog
    .getByRole("button", { name: "选择筛选结果", exact: true })
    .click();
  await search.fill("no-such-skill");
  await expect(
    dialog.getByText("没有筛选结果，已选项目仍保留。", { exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole("button", { name: "预览导入 · 1", exact: true })
    .click();
  expect(writes(await snapshot(page), "import_skills_from_apps")).toHaveLength(
    0,
  );
  await dialog.getByRole("button", { name: "重新选择", exact: true }).click();
  await dialog
    .getByRole("button", { name: "预览导入 · 1", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "确认导入 · 1", exact: true })
    .click();
  await expect(dialog.getByText("导入完成", { exact: true })).toBeVisible();
  const imported = await snapshot(page);
  const imports = writes(imported, "import_skills_from_apps");
  expect(imports).toHaveLength(1);
  expect(imports[0].payload.imports).toEqual([
    {
      directory: "sprint-directory-69",
      apps: expect.objectContaining({ claude: true, codex: false }),
    },
  ]);
  expect(imported.skillIds).toEqual(["local:sprint-directory-69"]);
  expect(imported.unmanagedDirectories).toEqual(
    initial.unmanagedDirectories.slice(0, 69),
  );
  const importIndex = imported.events.findIndex(
    (event) => event.command === "import_skills_from_apps",
  );
  expect(
    imported.events
      .slice(importIndex + 1)
      .some(
        (event) =>
          event.command === "get_installed_skills" &&
          Array.isArray(event.observed) &&
          event.observed.length === 1,
      ),
  ).toBe(true);
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole("button", { name: "批量分配", exact: true }).click();
  const bulk = page.getByRole("dialog", {
    name: "Skills 批量分配",
    exact: true,
  });
  await expect(
    bulk.getByRole("button", { name: "预览所选 · 0", exact: true }),
  ).toBeDisabled();
  await bulk
    .getByRole("combobox", { name: "目标软件", exact: true })
    .selectOption("codex");
  await bulk.getByRole("button", { name: "选择筛选结果", exact: true }).click();
  await bulk.getByRole("button", { name: "预览所选 · 1", exact: true }).click();
  expect(writes(await snapshot(page), "toggle_skill_app")).toHaveLength(0);
  await bulk.getByRole("button", { name: "确认执行 · 1", exact: true }).click();
  await expect(
    bulk.getByText("完成：分配状态已读回", { exact: true }),
  ).toBeVisible();
  const finished = await snapshot(page);
  expect(
    writes(finished, "toggle_skill_app").map((event) => event.payload),
  ).toEqual([{ id: "local:sprint-directory-69", app: "codex", enabled: true }]);
  expect(finished.skillApps["local:sprint-directory-69"]).toEqual({
    ...imported.skillApps["local:sprint-directory-69"],
    codex: true,
  });
  expect(finished.unmanagedDirectories).toEqual(
    initial.unmanagedDirectories.slice(0, 69),
  );
  const finalSkillWrite = finished.events
    .map((event) => event.command)
    .lastIndexOf("toggle_skill_app");
  expect(
    finished.events
      .slice(finalSkillWrite + 1)
      .some((event) => event.command === "get_installed_skills"),
  ).toBe(true);
  await captureSprintEvidence(page, "sprint-skills-import-bulk-900x600.png");
  await expectHealthyPage(page, health);
});

test("MCP preserves partial results and retries only failures, including a mismatched readback", async ({
  page,
}) => {
  await installSprintFixture(page, "mcp");
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/mcp");
  await page.getByRole("button", { name: "批量分配", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "MCP 批量分配",
    exact: true,
  });
  await expect(
    dialog.getByRole("button", { name: "预览所选 · 0", exact: true }),
  ).toBeDisabled();
  await dialog
    .getByRole("combobox", { name: "目标软件", exact: true })
    .selectOption("codex");
  await dialog
    .getByRole("button", { name: "选择筛选结果", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "预览所选 · 3", exact: true })
    .click();
  expect(writes(await snapshot(page), "toggle_mcp_app")).toHaveLength(0);
  await dialog
    .getByRole("button", { name: "确认执行 · 3", exact: true })
    .click();
  const row = (name: string) =>
    dialog
      .locator("article")
      .filter({ has: page.getByText(`Sprint MCP ${name}`, { exact: true }) });
  await expect(
    row("success").getByText("完成：分配状态已读回", { exact: true }),
  ).toBeVisible();
  await expect(row("throw").getByText(/^失败：/)).toBeVisible();
  await expect(row("mismatch").getByText(/^失败：/)).toBeVisible();
  await expect(
    row("mismatch").getByText("完成：分配状态已读回", { exact: true }),
  ).toHaveCount(0);
  const first = await snapshot(page);
  expect(
    writes(first, "toggle_mcp_app").map((event) => event.payload.serverId),
  ).toEqual(["success", "throw", "mismatch"]);
  expect(first.mcpApps.success.codex).toBe(true);
  expect(first.mcpApps.throw.codex).toBe(false);
  expect(first.mcpApps.mismatch.codex).toBe(false);
  await dialog
    .getByRole("button", { name: "选择未完成项重新预览", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "预览所选 · 2", exact: true })
    .click();
  await expect(
    dialog.getByText("Sprint MCP success", { exact: true }),
  ).toHaveCount(0);
  await dialog
    .getByRole("button", { name: "确认执行 · 2", exact: true })
    .click();
  await expect(
    dialog.getByText("完成：分配状态已读回", { exact: true }),
  ).toHaveCount(2);
  const last = await snapshot(page);
  expect(
    writes(last, "toggle_mcp_app")
      .slice(3)
      .map((event) => event.payload.serverId),
  ).toEqual(["throw", "mismatch"]);
  expect(last.mcpApps).toEqual(
    Object.fromEntries(
      Object.entries(first.mcpApps).map(([id, state]) => [
        id,
        { ...state, codex: true },
      ]),
    ),
  );
  const lastWrite = last.events
    .map((event) => event.command)
    .lastIndexOf("toggle_mcp_app");
  expect(
    last.events
      .slice(lastWrite + 1)
      .some((event) => event.command === "get_mcp_servers"),
  ).toBe(true);
  await captureSprintEvidence(page, "sprint-mcp-retry-900x600.png");
  await expectHealthyPage(page, health);
});

test("linked observation explains protection and keeps the ordinary target writable at 900x600", async ({
  page,
}) => {
  await installSprintFixture(page, "linked");
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/skills");
  await expect(
    page.getByRole("heading", { name: "Observed Linked Skill", exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/检测到链接目录：/)).toBeVisible();
  await expect(page.getByText(/只读目标：Claude/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "卸载", exact: true }),
  ).toBeDisabled();
  const linked = page.getByRole("switch", {
    name: "Claude Code Skill 分配",
    exact: true,
  });
  const ordinary = page.getByRole("switch", {
    name: "Codex Skill 分配",
    exact: true,
  });
  await expect(linked).toBeDisabled();
  await expect(ordinary).toBeEnabled();
  expect(writes(await snapshot(page), "toggle_skill_app")).toHaveLength(0);
  const linkedNotice = page.getByText(/检测到链接目录：/);
  await linkedNotice.scrollIntoViewIfNeeded();
  await expect(linkedNotice).toBeInViewport();
  await captureSprintEvidence(page, "sprint-linked-notice-900x600.png");
  await ordinary.check();
  await expect(ordinary).toBeChecked();
  await expect(ordinary).toBeEnabled();
  await expect(linked).toBeDisabled();
  const state = await snapshot(page);
  expect(
    writes(state, "toggle_skill_app").map((event) => event.payload),
  ).toEqual([{ id: "local:sprint-linked", app: "codex", enabled: true }]);
  expect(state.skillApps["local:sprint-linked"].claude).toBe(true);
  await expectNoHorizontalOverflow(page);
  await captureSprintEvidence(page, "sprint-linked-observation-900x600.png");
  await expectHealthyPage(page, health);
});

test("Sessions shows four persisted facts without promoting a request stage by user attestation", async ({
  page,
}) => {
  await installSprintFixture(page, "sessions");
  const health = monitorPageHealth(page);
  await openRendererPage(page, "/sessions");
  await page.getByRole("button", { name: /^恢复记录 \(1\)$/ }).click();
  await page
    .getByRole("complementary", { name: "会话列表", exact: true })
    .getByRole("listitem")
    .click();
  const stages = page.getByRole("region", {
    name: "会话能力与恢复阶段",
    exact: true,
  });
  for (const label of ["包导入", "目标写入", "目标读回", "实机闭环"])
    await expect(stages.getByText(label, { exact: true })).toBeVisible();
  await expect(
    stages.getByText("系统已验证下一轮请求；真实回复尚未核验", { exact: true }),
  ).toBeVisible();
  await expect(
    stages.getByText("系统已验证真实模型回复", { exact: true }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "标记：我已手动续聊", exact: true })
    .click();
  await page.getByRole("button", { name: "确认记录", exact: true }).click();
  await expect(page.getByText(/用户自报标记：已手动确认续聊/)).toBeVisible();
  await expect(
    stages.getByText("系统已验证下一轮请求；真实回复尚未核验", { exact: true }),
  ).toBeVisible();
  await expect(
    stages.getByText("系统已验证真实模型回复", { exact: true }),
  ).toHaveCount(0);
  const state = await snapshot(page);
  expect(state.attemptStage).toBe("nextTurnRequestVerified");
  expect(state.hasAttestation).toBe(true);
  expect(writes(state, "record_user_attestation")).toHaveLength(1);
  await expect(
    page.getByText("已记录手动续聊标记", { exact: true }),
  ).toBeHidden();
  for (const [index, label] of [
    "包导入",
    "目标写入",
    "目标读回",
    "实机闭环",
  ].entries()) {
    const fact = stages.getByText(label, { exact: true });
    const value = fact.locator("xpath=following-sibling::dd[1]");
    await value.scrollIntoViewIfNeeded();
    await value.evaluate((element) =>
      element.scrollIntoView({ block: "center", inline: "nearest" }),
    );
    await expect(fact).toBeInViewport();
    await expect(value).toBeInViewport({ ratio: 1 });
    await captureSprintEvidence(
      page,
      `sprint-session-fact-${index + 1}-900x600.png`,
    );
  }
  const requestFact = stages.getByText(
    "系统已验证下一轮请求；真实回复尚未核验",
    { exact: true },
  );
  await requestFact.scrollIntoViewIfNeeded();
  await expect(requestFact).toBeInViewport();
  await captureSprintEvidence(
    page,
    "sprint-session-facts-loop-view-900x600.png",
  );
  await expectHealthyPage(page, health);
});
