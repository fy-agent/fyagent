import { expect, test, type Page } from "@playwright/test";

import { installRichTauriFeatureFixture } from "./support/features";

const contentDigest = `fyc1:${"a".repeat(64)}`;
const snapshotId = `fys1:${"b".repeat(64)}`;

async function installSessionMigrationFixture(page: Page): Promise<void> {
  await installRichTauriFeatureFixture(page);
  await page.addInitScript(
    ({ digest, snapshot }) => {
      const firstSession = {
        providerId: "codex",
        sessionId: "source-session-a",
        title: "相同正文 · 来源 A",
        summary: "同正文不能折叠",
        projectDir: "/source/private-a",
        lastActiveAt: 1_795_478_400_000,
        sourcePath: "/isolated/codex-a.jsonl",
      };
      const secondSession = {
        ...firstSession,
        sessionId: "source-session-b",
        title: "相同正文 · 来源 B",
        projectDir: "/source/private-b",
        sourcePath: "/isolated/codex-b.jsonl",
      };
      const firstOrigin = {
        originId: "11111111-1111-4111-8111-111111111111",
        providerId: "codex",
        sessionId: firstSession.sessionId,
        cliVersion: "0.154.0",
      };
      const messages = [
        {
          seq: 0,
          kind: "userText",
          text: "A：这一问被用户中断，没有答复。",
        },
        {
          seq: 1,
          kind: "userText",
          text: "B：重新提问，只回答 B。正文路径 /Users/alice/demo 与 D:\\work\\demo。",
        },
        {
          seq: 2,
          kind: "assistantFinal",
          text: "B 的最终答复。FINAL-T2-KEEP",
        },
        { seq: 3, kind: "userText", text: "C：尾部未完成输入。" },
      ];
      const firstPreview = {
        snapshotId: snapshot,
        contentDigest: digest,
        origin: firstOrigin,
        workspaceLabel: "private-a",
        messages,
        extraction: {
          ruleId: "codex.rollout.phase-v1",
          ruleVerifiedVersions: ["=0.154.0"],
          openUserMessages: [0, 3],
          omitted: {
            toolEvents: 2,
            reasoningBlocks: 1,
            commentaryMessages: 1,
            attachments: 1,
            runtimeInjections: 2,
            unknownBlocks: 0,
          },
          sourcePathFamily: "posix",
        },
      };
      const secondPreview = {
        ...structuredClone(firstPreview),
        snapshotId: `fys1:${"d".repeat(64)}`,
        origin: {
          ...firstOrigin,
          originId: "44444444-4444-4444-8444-444444444444",
          sessionId: secondSession.sessionId,
        },
        workspaceLabel: "private-b",
      };
      const attempt = {
        attemptId: "22222222-2222-4222-8222-222222222222",
        requestId: "33333333-3333-4333-8333-333333333333",
        snapshotId: snapshot,
        requestKind: "defaultImport",
        idempotencySlot: `fyslot1:${"c".repeat(64)}`,
        contentDigest: digest,
        origin: firstOrigin,
        targetProviderId: "codex",
        targetStoreId: "isolated-target-store",
        targetNativeNonce: "isolated-native-nonce",
        targetNativeId: "isolated-native-id",
        targetWorkspace: "/tmp/fyagent-session-migration",
        stage: "nativeWritten",
        attemptCount: 1,
        userAttestation: null,
        createdAt: 1_795_478_400_000,
        updatedAt: 1_795_478_401_000,
      };

      const delegate = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, payload = {}) => {
        window.__FYAGENT_FEATURE_FIXTURE__.calls.push({ command, payload });
        switch (command) {
          case "list_sessions":
            return structuredClone([firstSession, secondSession]);
          case "get_session_messages":
            return [];
          case "preview_session_migration":
            return structuredClone(
              payload.sourcePath === secondSession.sourcePath
                ? secondPreview
                : firstPreview,
            );
          case "probe_local_provider":
            return {
              providerId: payload.providerId,
              installed: true,
              detectedVersion:
                payload.providerId === "codex" ? "0.154.0" : "0.0.0",
              extractionSupported: payload.providerId === "codex",
              writeSupported: false,
              reasonCode: "providerVersionUnsupported",
            };
          case "get_release_capability_matrix":
            return [
              {
                providerId: "codex",
                extractionRule: {
                  ruleId: "codex.rollout.phase-v1",
                  verifiedVersions: ["=0.154.0"],
                },
                writeStrategy: null,
                verifiedStages: [],
              },
            ];
          case "list_restore_attempts":
            return [structuredClone(attempt)];
          case "record_user_attestation":
            return {
              ...structuredClone(attempt),
              userAttestation: {
                attestedAt: Date.now(),
                claimedStage: payload.claimedStage,
                note: payload.note,
              },
            };
          case "pick_directory":
            return "/tmp/new-target-workspace";
          case "open_restored_session":
            return true;
          default:
            return delegate(command, payload);
        }
      };
    },
    { digest: contentDigest, snapshot: snapshotId },
  );
}

test("keeps same-body sources distinct and preserves consecutive unfinished users", async ({
  page,
}) => {
  await installSessionMigrationFixture(page);
  await page.goto("/#/sessions");

  const list = page.getByRole("complementary", { name: "会话列表" });
  await expect(list.getByRole("listitem")).toHaveCount(2);
  await expect(
    page.getByText("相同正文 · 来源 A", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("相同正文 · 来源 B", { exact: true }),
  ).toBeVisible();
  await page.getByText("相同正文 · 来源 A", { exact: true }).click();
  await expect(
    page.getByText("原标签: private-a", { exact: true }),
  ).toBeVisible();
  await page.getByText("相同正文 · 来源 B", { exact: true }).click();
  await expect(
    page.getByText("原标签: private-b", { exact: true }),
  ).toBeVisible();

  const previewSourcePaths = await page.evaluate(() =>
    window.__FYAGENT_FEATURE_FIXTURE__.calls
      .filter((call) => call.command === "preview_session_migration")
      .map((call) => call.payload.sourcePath),
  );
  expect(previewSourcePaths).toEqual(
    expect.arrayContaining([
      "/isolated/codex-a.jsonl",
      "/isolated/codex-b.jsonl",
    ]),
  );

  for (const body of [
    "A：这一问被用户中断，没有答复。",
    "B：重新提问，只回答 B。正文路径 /Users/alice/demo 与 D:\\work\\demo。",
    "B 的最终答复。FINAL-T2-KEEP",
    "C：尾部未完成输入。",
  ]) {
    await expect(page.getByText(body, { exact: true })).toBeVisible();
  }
  await expect(
    page.getByText("轮次未完成 · 等待模型答复中被中断", { exact: true }),
  ).toHaveCount(2);
  await expect(
    page.getByText(/LEAK_(?:TOOL|PROGRESS|REASONING|FILE)/u),
  ).toHaveCount(0);
});

test("cannot promote disabled capability by attestation or workspace choice", async ({
  page,
}) => {
  await installSessionMigrationFixture(page);
  await page.goto("/#/sessions");

  await page.getByText("相同正文 · 来源 A", { exact: true }).click();
  await expect(
    page.getByText("当前版本的恢复能力尚未验证", { exact: true }),
  ).toBeVisible();
  const restore = page.getByRole("button", { name: "在目标软件中恢复" });
  await expect(restore).toBeDisabled();

  await page.getByRole("button", { name: "标记：我已手动续聊" }).last().click();
  await expect(page.getByRole("note")).toContainText("仅记录你主观确认");
  await page.getByRole("button", { name: "确认记录" }).click();

  await expect(
    page.getByText("当前版本的恢复能力尚未验证", { exact: true }),
  ).toBeVisible();
  await expect(restore).toBeDisabled();
});

test("keeps the complete legacy memory editor reachable as an auxiliary entry", async ({
  page,
}) => {
  await installSessionMigrationFixture(page);
  await page.goto("/#/sessions");

  const memory = page.getByRole("link", { name: /记忆模块/u }).last();
  await expect(memory).toBeVisible();
  await memory.click();
  await expect(page).toHaveURL(/#\/memory$/u);
  await expect(page.getByTestId("memory-page")).toBeVisible();
});
