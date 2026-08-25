import {
  act,
  fireEvent,
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { PromptsPage } from "@/v2/pages/prompts/Page";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { PrimaryBlockerProvider } from "@/v2/shared/ui/PrimaryBlocker";
import {
  PROMPT_APP_IDS,
  type ManagedPrompt,
  type PromptAppId,
} from "@/v2/shared/features/types";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

function prompt(
  id: string,
  name: string,
  enabled = false,
  content = `${name} content`,
): ManagedPrompt {
  return {
    id,
    name,
    description: `${name} description`,
    content,
    enabled,
    createdAt: 1_700_000_000,
    updatedAt: 1_700_000_100,
  };
}

function emptyStores(): Record<PromptAppId, ManagedPrompt[]> {
  return {
    claude: [],
    codex: [],
    gemini: [],
    grokbuild: [],
    opencode: [],
    openclaw: [],
    hermes: [],
  };
}

function clonePrompt(value: ManagedPrompt): ManagedPrompt {
  return { ...value };
}

function statefulPorts(
  initial: Partial<Record<PromptAppId, ManagedPrompt[]>> = {},
) {
  const stores: Record<PromptAppId, ManagedPrompt[]> = emptyStores();
  for (const app of PROMPT_APP_IDS) {
    stores[app] = (initial[app] ?? []).map(clonePrompt);
  }
  const liveFiles: Record<PromptAppId, string | null> = Object.fromEntries(
    PROMPT_APP_IDS.map((app) => [app, null]),
  ) as Record<PromptAppId, string | null>;
  const ports = createBrowserFeaturePorts();
  ports.prompts.getAll = vi.fn(async (app: PromptAppId) =>
    stores[app].map(clonePrompt),
  );
  ports.prompts.getCurrentFileContent = vi.fn(
    async (app: PromptAppId) => liveFiles[app],
  );
  ports.prompts.upsert = vi.fn(
    async (app: PromptAppId, value: ManagedPrompt) => {
      const current = stores[app];
      const index = current.findIndex((candidate) => candidate.id === value.id);
      if (index >= 0) current[index] = clonePrompt(value);
      else current.unshift(clonePrompt(value));
      if (value.enabled) liveFiles[app] = value.content;
      else if (!current.some((candidate) => candidate.enabled))
        liveFiles[app] = null;
    },
  );
  ports.prompts.enable = vi.fn(async (app: PromptAppId, id: string) => {
    stores[app] = stores[app].map((candidate) => ({
      ...candidate,
      enabled: candidate.id === id,
    }));
    liveFiles[app] =
      stores[app].find((candidate) => candidate.id === id)?.content ?? null;
  });
  ports.prompts.delete = vi.fn(async (app: PromptAppId, id: string) => {
    stores[app] = stores[app].filter((candidate) => candidate.id !== id);
  });
  ports.prompts.importFromFile = vi.fn(async (app: PromptAppId) => {
    const imported = prompt(`${app}-imported`, `${app} imported`);
    stores[app].unshift(imported);
    return imported.id;
  });
  return { ports, stores, liveFiles };
}

function renderPrompts(ports: FeaturePorts) {
  const router = createMemoryRouter(
    [
      {
        path: "/prompts",
        element: (
          <PrimaryBlockerProvider>
            <PromptsPage />
          </PrimaryBlockerProvider>
        ),
      },
      { path: "/memory", element: <h1>记忆目标页</h1> },
    ],
    { initialEntries: ["/prompts"] },
  );
  render(
    <FeatureProvider ports={ports}>
      <RouterProvider router={router} />
    </FeatureProvider>,
  );
  return router;
}

describe("PromptsPage native business management", () => {
  it("loads Claude by default and keeps all seven applications independent", async () => {
    const { ports } = statefulPorts({
      claude: [prompt("claude-one", "Claude rule", true)],
      codex: [prompt("codex-one", "Codex rule")],
    });
    const user = userEvent.setup();
    renderPrompts(ports);

    const page = screen.getByTestId("prompts-page");
    const pageHeader = page.querySelector<HTMLElement>(
      ":scope > .fy-feature-header",
    );
    expect(pageHeader).not.toBeNull();
    expect(
      within(pageHeader!).getByRole("heading", {
        level: 1,
        name: "提示词管理",
      }),
    ).toBeVisible();
    expect(
      within(pageHeader!).getByRole("button", { name: "从文件导入" }),
    ).toBeVisible();
    expect(
      within(pageHeader!).getByRole("button", { name: "新建提示词" }),
    ).toBeVisible();
    expect(
      await screen.findByRole("heading", { name: "Claude rule" }),
    ).toBeVisible();
    const editor = screen.getByRole("region", { name: "提示词详情" });
    const editorHead = editor.querySelector<HTMLElement>(
      ".fy-prompts-editor-head",
    );
    expect(editorHead).not.toBeNull();
    expect(
      within(editorHead!).getByRole("switch", { name: "停用Claude rule" }),
    ).toBeVisible();
    expect(
      within(editorHead!).getByRole("button", { name: "保存" }),
    ).toBeVisible();
    expect(
      within(editorHead!).getByRole("button", { name: "删除" }),
    ).toBeVisible();
    expect(screen.getByRole("textbox", { name: "内容" })).toBeVisible();
    expect(screen.getByRole("textbox", { name: "内容" })).toHaveValue(
      "Claude rule content",
    );
    expect(document.querySelector(".fy-prompts-live")).not.toHaveAttribute(
      "open",
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByTestId("prompts-page")).toHaveAttribute(
      "data-data-source",
      "native",
    );
    expect(
      screen.queryByText(/前端原型|注入目标|已保存到前端预览/),
    ).not.toBeInTheDocument();
    expect(screen.getByTestId("prompt-app-claude")).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(
      await within(screen.getByTestId("prompt-app-claude")).findByText(
        "1 条已启用",
      ),
    ).toBeVisible();
    expect(
      within(screen.getByTestId("prompt-app-codex")).getByText("0 条已启用"),
    ).toBeVisible();
    expect(
      PROMPT_APP_IDS.map((id) => screen.getByTestId(`prompt-app-${id}`)),
    ).toHaveLength(7);
    expect([...PROMPT_APP_IDS]).toEqual([
      "grokbuild",
      "codex",
      "claude",
      "opencode",
      "gemini",
      "openclaw",
      "hermes",
    ]);
    expect(
      Array.from(
        document.querySelectorAll('[data-testid^="prompt-app-"]'),
      ).map((node) => node.getAttribute("data-testid")),
    ).toEqual(PROMPT_APP_IDS.map((id) => `prompt-app-${id}`));
    expect(screen.getByTestId("prompt-app-grokbuild")).toHaveTextContent(
      "Grok Build",
    );
    expect(screen.getByTestId("prompt-app-claude")).toHaveTextContent(
      "Claude Code",
    );

    await user.click(screen.getByTestId("prompt-app-codex"));
    expect(
      await screen.findByRole("heading", { name: "Codex rule" }),
    ).toBeVisible();
    expect(screen.queryByText("Claude rule")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("prompt-app-claude"));
    expect(
      await screen.findByRole("heading", { name: "Claude rule" }),
    ).toBeVisible();
  });

  it("searches authoritative records and distinguishes no results", async () => {
    const { ports } = statefulPorts({
      claude: [
        prompt("review", "Review rule", false, "inspect regressions"),
        prompt("reply", "Reply rule", false, "answer briefly"),
      ],
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Review rule" });

    await user.type(
      screen.getByRole("searchbox", { name: "搜索提示词" }),
      "regressions",
    );
    expect(screen.getAllByText("Review rule").length).toBeGreaterThan(0);
    expect(screen.queryByText("Reply rule")).not.toBeInTheDocument();
    await user.clear(screen.getByRole("searchbox", { name: "搜索提示词" }));
    await user.type(
      screen.getByRole("searchbox", { name: "搜索提示词" }),
      "missing",
    );
    expect(screen.getByText("没有匹配的提示词")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "清空搜索" }));
    expect(await screen.findByText("Reply rule")).toBeVisible();
  });

  it("keeps the selected prompt while search hides it from the list", async () => {
    const { ports } = statefulPorts({
      claude: [
        prompt("review", "Review rule", false, "inspect regressions"),
        prompt("reply", "Reply rule", false, "answer briefly"),
      ],
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Review rule" });
    await user.click(screen.getByRole("button", { name: /Reply rule/ }));
    expect(
      await screen.findByRole("heading", { name: "Reply rule" }),
    ).toBeVisible();

    await user.type(
      screen.getByRole("searchbox", { name: "搜索提示词" }),
      "regressions",
    );
    const library = screen.getByRole("region", { name: "提示词列表" });
    expect(within(library).queryByText("Reply rule")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Reply rule" })).toBeVisible();
    expect(screen.getByRole("textbox", { name: "内容" })).toHaveValue(
      "answer briefly",
    );

    await user.clear(screen.getByRole("searchbox", { name: "搜索提示词" }));
    await user.type(
      screen.getByRole("searchbox", { name: "搜索提示词" }),
      "missing",
    );
    expect(screen.getByRole("heading", { name: "Reply rule" })).toBeVisible();
    expect(screen.getByText("当前编辑的提示词不在搜索结果中。")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "清空搜索" }),
    ).not.toBeInTheDocument();
  });

  it("creates and edits inline with legacy ids and timestamps", async () => {
    const { ports, stores } = statefulPorts({
      claude: [prompt("existing", "Existing")],
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Existing" });

    await user.click(screen.getByRole("button", { name: "新建提示词" }));
    expect(
      screen.getByRole("heading", { name: "新建 Claude Code 提示词" }),
    ).toBeVisible();
    await user.type(screen.getByRole("textbox", { name: "名称" }), "New rule");
    await user.type(
      screen.getByRole("textbox", { name: "描述" }),
      "New description",
    );
    await user.type(
      screen.getByRole("textbox", { name: "内容" }),
      "New content",
    );
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(
      await screen.findByRole("heading", { name: "New rule" }),
    ).toBeVisible();
    const created = stores.claude.find(
      (candidate) => candidate.name === "New rule",
    );
    expect(created?.id).toMatch(/^prompt-\d+$/);
    expect(created?.createdAt).toEqual(expect.any(Number));
    expect(created?.updatedAt).toEqual(expect.any(Number));
    expect(created?.enabled).toBe(false);

    const description = screen.getByRole("textbox", { name: "描述" });
    await user.clear(description);
    await user.type(description, "Updated description");
    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByRole("textbox", { name: "描述" })).toHaveValue(
      "Updated description",
    );
    expect(await screen.findByText(/Updated description/)).toBeVisible();
    expect(
      stores.claude.find((candidate) => candidate.id === created?.id)
        ?.description,
    ).toBe("Updated description");
  });

  it("imports, enables mutually, rereads the live file, disables, and deletes", async () => {
    const { ports, stores } = statefulPorts({
      claude: [
        prompt("first", "First", true, "first live"),
        prompt("second", "Second"),
      ],
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "First" });

    await user.click(
      screen.getByRole("button", { name: /Second description/ }),
    );
    expect(
      await screen.findByRole("heading", { name: "Second" }),
    ).toBeVisible();
    await user.click(screen.getByRole("switch", { name: "启用Second" }));
    expect(
      await screen.findByRole("switch", { name: "停用Second" }),
    ).toBeChecked();
    expect(
      stores.claude.find((candidate) => candidate.id === "first")?.enabled,
    ).toBe(false);
    expect(screen.getByRole("textbox", { name: "当前使用的内容" })).toHaveValue(
      "Second content",
    );

    await user.click(screen.getByRole("switch", { name: "停用Second" }));
    expect(
      await screen.findByRole("switch", { name: "启用Second" }),
    ).not.toBeChecked();
    await user.click(screen.getByRole("button", { name: "删除" }));
    const confirm = screen.getByRole("dialog", { name: "删除 Second" });
    await user.click(within(confirm).getByRole("button", { name: "确认" }));
    await waitFor(() =>
      expect(screen.queryByText("Second")).not.toBeInTheDocument(),
    );

    await user.click(screen.getByRole("button", { name: "从文件导入" }));
    expect(
      await screen.findByRole("heading", { name: "claude imported" }),
    ).toBeVisible();
    expect(ports.prompts.importFromFile).toHaveBeenCalledWith("claude");
  });

  it("rejects deleting an enabled prompt before invoking the backend", async () => {
    const { ports } = statefulPorts({
      claude: [prompt("active", "Active", true)],
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Active" });

    await user.click(screen.getByRole("button", { name: "删除" }));
    expect(
      screen.getByText("已启用提示词不能删除，请先停用后再删除"),
    ).toBeVisible();
    expect(ports.prompts.delete).not.toHaveBeenCalled();
  });

  it("keeps a missing import-file failure visible without inventing a record", async () => {
    const { ports } = statefulPorts();
    ports.prompts.importFromFile = vi.fn(async () => {
      throw new Error("提示词文件不存在");
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByText("Claude Code 还没有提示词");

    await user.click(screen.getAllByRole("button", { name: "从文件导入" })[0]);

    expect(
      await screen.findByText("提示词已从文件导入失败：请稍后重试。"),
    ).toBeVisible();
    expect(screen.queryByText("提示词文件不存在")).not.toBeInTheDocument();
    expect(screen.getByText("Claude Code 还没有提示词")).toBeVisible();
    expect(
      screen.queryByText("提示词已从文件导入", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("locks duplicate submissions until the authoritative reread completes", async () => {
    let resolveWrite!: () => void;
    const pendingWrite = new Promise<void>((resolve) => {
      resolveWrite = resolve;
    });
    const { ports } = statefulPorts({
      claude: [prompt("existing", "Existing")],
    });
    ports.prompts.upsert = vi.fn(() => pendingWrite);
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Existing" });
    const save = screen.getByRole("button", { name: "保存" });

    await user.click(save);
    expect(screen.getByRole("button", { name: "保存中…" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "保存中…" }));
    expect(ports.prompts.upsert).toHaveBeenCalledTimes(1);
    resolveWrite();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "保存" })).toBeEnabled(),
    );
  });

  it("keeps cached data and reports uncertainty when a post-write refresh fails", async () => {
    const { ports } = statefulPorts({
      claude: [prompt("existing", "Existing")],
    });
    let reads = 0;
    ports.prompts.getAll = vi.fn(async () => {
      reads += 1;
      if (reads === 1) return [prompt("existing", "Existing")];
      throw new Error("refresh unavailable");
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Existing" });
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(
      await screen.findByText(
        /写入可能已完成，但状态刷新失败。已保留上一次成功读取的数据/,
        undefined,
        {
          timeout: 5_000,
        },
      ),
    ).toBeVisible();
    expect(screen.getAllByText("Existing").length).toBeGreaterThan(0);
    expect(screen.queryByText("提示词已保存")).not.toBeInTheDocument();
  });

  it("does not show success or replace the baseline after a failed save", async () => {
    const { ports } = statefulPorts({
      claude: [prompt("existing", "Existing")],
    });
    ports.prompts.upsert = vi.fn(async () => {
      throw new Error("save rejected");
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Existing" });
    const name = screen.getByRole("textbox", { name: "名称" });
    await user.clear(name);
    await user.type(name, "Changed");
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(
      await screen.findByText("提示词已保存失败：请稍后重试。"),
    ).toBeVisible();
    expect(screen.queryByText("save rejected")).not.toBeInTheDocument();
    expect(name).toHaveValue("Changed");
    expect(
      screen.queryByText("提示词已保存", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("uses the shared confirm dialog before discarding an app-switch draft", async () => {
    const { ports } = statefulPorts({
      claude: [prompt("claude-one", "Claude rule")],
      codex: [prompt("codex-one", "Codex rule")],
    });
    const user = userEvent.setup();
    renderPrompts(ports);
    await screen.findByRole("heading", { name: "Claude rule" });
    await user.type(screen.getByRole("textbox", { name: "描述" }), " dirty");

    await user.click(screen.getByTestId("prompt-app-codex"));
    const confirm = screen.getByRole("dialog", {
      name: "放弃未保存的提示词更改",
    });
    await user.click(within(confirm).getByRole("button", { name: "取消" }));
    expect(screen.getByTestId("prompt-app-claude")).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(screen.getByRole("heading", { name: "Claude rule" })).toBeVisible();

    await user.click(screen.getByTestId("prompt-app-codex"));
    await user.click(
      within(
        screen.getByRole("dialog", { name: "放弃未保存的提示词更改" }),
      ).getByRole("button", { name: "确认" }),
    );
    expect(
      await screen.findByRole("heading", { name: "Codex rule" }),
    ).toBeVisible();
  });

  it("blocks route navigation with the same dirty-state confirmation", async () => {
    const { ports } = statefulPorts({
      claude: [prompt("existing", "Existing")],
    });
    const router = renderPrompts(ports);
    const user = userEvent.setup();
    await screen.findByRole("heading", { name: "Existing" });
    await user.type(screen.getByRole("textbox", { name: "内容" }), " dirty");

    await act(async () => {
      await router.navigate("/memory");
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    const confirm = await screen.findByRole("dialog", {
      name: "放弃未保存的提示词更改",
    });
    await user.click(within(confirm).getByRole("button", { name: "取消" }));
    expect(router.state.location.pathname).toBe("/prompts");
    await act(async () => {
      await router.navigate("/memory");
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    await user.click(
      within(
        await screen.findByRole("dialog", { name: "放弃未保存的提示词更改" }),
      ).getByRole("button", { name: "确认" }),
    );
    expect(
      await screen.findByRole("heading", { name: "记忆目标页" }),
    ).toBeVisible();
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  });

  it("distinguishes browser native-only, real empty data, and read errors", async () => {
    const nativePorts = createBrowserFeaturePorts();
    renderPrompts(nativePorts);
    expect(
      await screen.findByText("桌面能力不可用", undefined, { timeout: 5_000 }),
    ).toBeVisible();
    expect(screen.queryByText(/Claude rule|示例/)).not.toBeInTheDocument();
    cleanup();

    const empty = statefulPorts();
    renderPrompts(empty.ports);
    expect(await screen.findByText("Claude Code 还没有提示词")).toBeVisible();
    cleanup();

    const failing = statefulPorts();
    failing.ports.prompts.getAll = vi.fn(async () => {
      throw new Error("database unavailable");
    });
    renderPrompts(failing.ports);
    expect(
      await screen.findByText("无法加载 Claude Code 提示词", undefined, {
        timeout: 5_000,
      }),
    ).toBeVisible();
    expect(screen.getByText("请稍后重试。")).toBeVisible();
    expect(screen.queryByText("database unavailable")).not.toBeInTheDocument();
  });
});
