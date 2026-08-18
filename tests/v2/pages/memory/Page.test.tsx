import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  createMemoryRouter,
  RouterProvider,
  type RouteObject,
} from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { MemoryPage } from "@/v2/pages/memory/Page";

const memoryTestRoutes: RouteObject[] = [
  { path: "/memory", element: <MemoryPage /> },
  { path: "/other", element: <main>其他页面</main> },
];

type TestRouter = ReturnType<typeof createMemoryRouter>;

function renderMemoryPage(): TestRouter {
  const router = createMemoryRouter(memoryTestRoutes, {
    initialEntries: ["/memory"],
  });
  render(<RouterProvider router={router} />);
  return router;
}

async function selectClaudeLongTerm(user: ReturnType<typeof userEvent.setup>) {
  await user.click(
    screen.getByRole("button", { name: /Claude Code · 长期记忆/ }),
  );
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("MemoryPage local-agent prototype", () => {
  it("renders three source lifecycles, four verified targets, and a truthful prototype boundary", async () => {
    const user = userEvent.setup();
    renderMemoryPage();

    expect(screen.getByRole("heading", { name: /^记忆$/ })).toBeVisible();
    expect(
      screen.getByText(
        "用匿名化结构预览 6 个工具、8 个 Agent 实例的记忆来源",
      ),
    ).toBeVisible();
    expect(
      screen.getByText("前端原型 · 未读取或写入本机文件"),
    ).toBeVisible();
    expect(
      screen.getByRole("tab", { name: "长期记忆", selected: true }),
    ).toBeVisible();
    expect(screen.getByRole("tab", { name: "每日记录" })).toBeVisible();
    expect(screen.getByRole("tab", { name: "会话记录" })).toBeVisible();
    expect(screen.getByTestId("memory-library")).toBeVisible();
    expect(screen.getByTestId("memory-editor")).toBeVisible();
    expect(screen.getByTestId("memory-inspector")).toBeVisible();
    expect(screen.getByTestId("memory-page")).toHaveAttribute(
      "data-data-source",
      "prototype",
    );

    await selectClaudeLongTerm(user);
    expect(screen.getAllByRole("checkbox")).toHaveLength(4);
    expect(
      screen.queryByRole("checkbox", { name: /同步到Codex/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("checkbox", { name: /同步到OpenCode/ }),
    ).not.toBeInTheDocument();
    expect(
      within(screen.getByTestId("memory-inspector")).getByText("已存在"),
    ).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: /Gemini CLI · 长期记忆入口/ }),
    );
    expect(
      within(screen.getByTestId("memory-inspector")).getByText("未发现"),
    ).toBeVisible();
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });

  it("treats content and targets as one saved revision and creates only pending preview tasks", async () => {
    const user = userEvent.setup();
    renderMemoryPage();
    await selectClaudeLongTerm(user);

    const saveButton = screen.getByRole("button", { name: "保存" });
    expect(saveButton).toBeDisabled();
    expect(screen.getByText("r1")).toBeVisible();

    await user.type(
      screen.getByRole("textbox", { name: "记忆内容" }),
      "\n- 新的可复用经验",
    );
    expect(saveButton).toBeEnabled();
    await user.click(saveButton);
    expect(
      screen.getByText("已保存到前端预览；尚未写入本机文件"),
    ).toBeVisible();
    expect(screen.getByText("r2")).toBeVisible();

    await user.click(
      screen.getByRole("checkbox", {
        name: "同步到OpenClaw默认工作区 · main + utility",
      }),
    );
    expect(
      within(screen.getByTestId("memory-inspector")).getByText(
        "修改待保存",
      ),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "生成 2 个同步预览任务" }),
    ).toBeDisabled();
    expect(
      screen.getByText("请先保存当前修改，再生成同步预览"),
    ).toBeVisible();

    await user.click(saveButton);
    expect(screen.getByText("r3")).toBeVisible();
    await user.click(
      screen.getByRole("button", { name: "生成 2 个同步预览任务" }),
    );
    expect(
      screen.getByText(
        "前端预览：已生成 2 个待执行任务；未写入本机文件",
      ),
    ).toBeVisible();
    const tasks = screen.getByTestId("memory-preview-tasks");
    expect(within(tasks).getAllByText("待执行 · 未写入")).toHaveLength(2);
    expect(within(tasks).getAllByText("基于修订 r3")).toHaveLength(2);
    for (const task of within(tasks).getAllByRole("listitem")) {
      expect(task).toHaveAttribute("data-preview-state", "pending");
      expect(task).toHaveAttribute("data-durable-state", "not-run");
    }
    expect(screen.queryByText("已同步")).not.toBeInTheDocument();

    await user.type(
      screen.getByRole("textbox", { name: "记忆标题" }),
      " 新版",
    );
    await user.click(saveButton);
    expect(screen.getByText("r4")).toBeVisible();
    expect(screen.queryByTestId("memory-preview-tasks")).not.toBeInTheDocument();
  });

  it("keeps tasks on a clean revision and when a later draft is discarded", async () => {
    const user = userEvent.setup();
    const confirm = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    renderMemoryPage();
    await selectClaudeLongTerm(user);

    await user.click(
      screen.getByRole("button", { name: "生成 1 个同步预览任务" }),
    );
    expect(screen.getByTestId("memory-preview-tasks")).toBeVisible();
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();

    await user.type(
      screen.getByRole("textbox", { name: "记忆标题" }),
      "未保存",
    );
    await user.click(
      screen.getByRole("button", { name: /Claude Code · 用户画像/ }),
    );
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "Claude Code · 长期记忆未保存",
    );
    expect(screen.getByTestId("memory-preview-tasks")).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: /Claude Code · 用户画像/ }),
    );
    expect(confirm).toHaveBeenCalledTimes(2);
    await user.click(
      screen.getByRole("button", { name: /Claude Code · 长期记忆/ }),
    );
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "Claude Code · 长期记忆",
    );
    expect(screen.getByText("r1")).toBeVisible();
    expect(screen.getByTestId("memory-preview-tasks")).toBeVisible();
  });

  it("ignores title-only surrounding whitespace without creating a revision", async () => {
    const user = userEvent.setup();
    renderMemoryPage();
    await selectClaudeLongTerm(user);

    await user.click(
      screen.getByRole("button", { name: "生成 1 个同步预览任务" }),
    );
    const tasks = screen.getByTestId("memory-preview-tasks");
    const title = screen.getByRole("textbox", { name: "记忆标题" });
    const saveButton = screen.getByRole("button", { name: "保存" });

    await user.clear(title);
    await user.type(title, "  Claude Code · 长期记忆  ");

    expect(saveButton).toBeDisabled();
    expect(screen.getByText("r1")).toBeVisible();
    expect(tasks).toBeVisible();
    expect(within(tasks).getByText("基于修订 r1")).toBeVisible();
  });

  it("keeps daily sources read-only and promotes an unsaved draft with full provenance", async () => {
    const user = userEvent.setup();
    renderMemoryPage();

    await user.click(screen.getByRole("tab", { name: "每日记录" }));
    expect(screen.getByRole("textbox", { name: "记忆内容" })).toHaveAttribute(
      "readonly",
    );
    expect(screen.getByRole("button", { name: "只读来源" })).toBeDisabled();
    expect(screen.getByText("可读写 · 可搜索")).toBeVisible();
    expect(screen.getByText("只读提炼")).toBeVisible();
    expect(screen.getByText("87 个文件")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "提炼为长期记忆" }));
    expect(
      screen.getByRole("tab", { name: "长期记忆", selected: true }),
    ).toBeVisible();
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "OpenClaw · 今日记录 · 提炼草稿",
    );
    expect(screen.getByText("前端草稿 · 未创建文件")).toBeVisible();
    expect(
      within(screen.getByTestId("memory-inspector")).getByText(
        "修改待保存",
      ),
    ).toBeVisible();
    expect(screen.getByText("r0")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "生成 0 个同步预览任务" }),
    ).toBeDisabled();
    expect(
      screen.getByText("已生成长期记忆草稿；原始记录保持不变"),
    ).toBeVisible();

    const provenance = screen.getByTestId("memory-provenance");
    expect(within(provenance).getByText("OpenClaw · 今日记录")).toBeVisible();
    expect(
      within(provenance).getByText("ID: openclaw-daily-latest"),
    ).toBeVisible();
    expect(within(provenance).getByText("OpenClaw")).toBeVisible();
    expect(within(provenance).getByText("toolId: openclaw")).toBeVisible();
    expect(
      within(provenance).getByText("默认工作区 · main + utility"),
    ).toBeVisible();
    expect(
      within(provenance).getByText("targetId: openclaw-default"),
    ).toBeVisible();
    expect(
      within(provenance).getByText(
        "~/.openclaw/workspace/memory/2026-08-12.md",
      ),
    ).toBeVisible();
    expect(within(provenance).getByText("今天 20:47")).toBeVisible();
    expect(
      within(provenance).getByText("按日期保存的工作痕迹与短期上下文"),
    ).toBeVisible();
    expect(within(provenance).getByText("提炼时间")).toBeVisible();
    expect(within(provenance).getByText("刚刚")).toBeVisible();
  });

  it("requires a promoted draft and its targets to be saved before preview", async () => {
    const user = userEvent.setup();
    renderMemoryPage();
    await user.click(screen.getByRole("tab", { name: "每日记录" }));
    await user.click(screen.getByRole("button", { name: "提炼为长期记忆" }));

    const saveButton = screen.getByRole("button", { name: "保存" });
    await user.click(saveButton);
    expect(screen.getByText("r1")).toBeVisible();
    const emptyPreviewButton = screen.getByRole("button", {
      name: "生成 0 个同步预览任务",
    });
    expect(emptyPreviewButton).toBeEnabled();
    await user.click(emptyPreviewButton);
    expect(
      screen.getByText("请先选择并保存至少一个同步目标"),
    ).toBeVisible();

    await user.click(
      screen.getByRole("checkbox", { name: "同步到Claude Code全局" }),
    );
    expect(
      screen.getByRole("button", { name: "生成 1 个同步预览任务" }),
    ).toBeDisabled();
    await user.click(saveButton);
    expect(screen.getByText("r2")).toBeVisible();
    await user.click(
      screen.getByRole("button", { name: "生成 1 个同步预览任务" }),
    );
    expect(screen.getByTestId("memory-preview-tasks")).toBeVisible();
    expect(
      screen.getByText(
        "前端预览：已生成 1 个待执行任务；未写入本机文件",
      ),
    ).toBeVisible();
  });

  it("keeps session sources read-only and preserves the original after discarding a promotion", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderMemoryPage();

    await user.click(screen.getByRole("tab", { name: "会话记录" }));
    await user.click(
      screen.getByRole("button", { name: /OpenCode · 会话数据库/ }),
    );
    expect(screen.getByRole("textbox", { name: "记忆内容" })).toHaveAttribute(
      "readonly",
    );
    expect(screen.getByText("1517 条")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "提炼为长期记忆" }));
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "OpenCode · 会话数据库 · 提炼草稿",
    );

    await user.click(screen.getByRole("tab", { name: "会话记录" }));
    expect(
      (screen.getByRole("textbox", {
        name: "记忆内容",
      }) as HTMLTextAreaElement).value,
    ).toContain("可用字段：project、directory、title、agent");
    expect(screen.getByText("1517 条")).toBeVisible();
    await user.click(screen.getByRole("tab", { name: "长期记忆" }));
    expect(
      screen.queryByRole("button", {
        name: /OpenCode · 会话数据库 · 提炼草稿/,
      }),
    ).not.toBeInTheDocument();
  });

  it("keeps Prompt-owned context read-only and routes ownership to Prompt", async () => {
    const user = userEvent.setup();
    renderMemoryPage();

    await user.click(
      screen.getByRole("button", { name: /OpenClaw · 上下文档案/ }),
    );
    expect(screen.getByRole("textbox", { name: "记忆内容" })).toHaveAttribute(
      "readonly",
    );
    expect(screen.getAllByText("由提示词管理").length).toBeGreaterThan(0);
    expect(screen.getByText("已存在")).toBeVisible();
    expect(
      within(screen.getByTestId("memory-inspector")).getByText(
        /请在提示词页面管理/,
      ),
    ).toBeVisible();
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });

  it("preserves query and selection independently for every category, including empty results", async () => {
    const user = userEvent.setup();
    renderMemoryPage();

    await user.click(
      screen.getByRole("button", { name: /Claude Code · 用户画像/ }),
    );
    await user.type(
      screen.getByRole("searchbox", { name: "搜索长期记忆" }),
      "Claude Code",
    );
    await user.click(screen.getByRole("tab", { name: "每日记录" }));
    await user.type(
      screen.getByRole("searchbox", { name: "搜索每日记录" }),
      "完全不存在的结果",
    );
    expect(screen.getByText("没有匹配的内容")).toBeVisible();

    await user.click(screen.getByRole("tab", { name: "会话记录" }));
    await user.click(
      screen.getByRole("button", { name: /OpenCode · 会话数据库/ }),
    );
    await user.type(
      screen.getByRole("searchbox", { name: "搜索会话来源" }),
      "OpenCode",
    );

    await user.click(screen.getByRole("tab", { name: "长期记忆" }));
    expect(
      screen.getByRole("searchbox", { name: "搜索长期记忆" }),
    ).toHaveValue("Claude Code");
    expect(
      screen.getByRole("button", { name: /Claude Code · 用户画像/ }),
    ).toHaveAttribute("aria-pressed", "true");

    await user.click(screen.getByRole("tab", { name: "每日记录" }));
    expect(
      screen.getByRole("searchbox", { name: "搜索每日记录" }),
    ).toHaveValue("完全不存在的结果");
    expect(screen.getByText("没有匹配的内容")).toBeVisible();

    await user.click(screen.getByRole("tab", { name: "会话记录" }));
    expect(
      screen.getByRole("searchbox", { name: "搜索会话来源" }),
    ).toHaveValue("OpenCode");
    expect(
      screen.getByRole("button", { name: /OpenCode · 会话数据库/ }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("guards category changes and simulated rescans without losing a rejected draft", async () => {
    const user = userEvent.setup();
    const confirm = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    renderMemoryPage();
    await selectClaudeLongTerm(user);
    await user.type(
      screen.getByRole("textbox", { name: "记忆标题" }),
      "未保存",
    );

    await user.click(screen.getByRole("tab", { name: "每日记录" }));
    expect(
      screen.getByRole("tab", { name: "长期记忆", selected: true }),
    ).toBeVisible();
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "Claude Code · 长期记忆未保存",
    );

    await user.click(screen.getByRole("button", { name: "重新扫描本机" }));
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "Claude Code · 长期记忆未保存",
    );
    await user.click(screen.getByRole("button", { name: "重新扫描本机" }));
    expect(screen.getByRole("textbox", { name: "记忆标题" })).toHaveValue(
      "Claude Code · 长期记忆",
    );
    expect(
      screen.getByText(
        "模拟扫描：6 个工具、8 个 Agent 实例；未访问本机文件",
      ),
    ).toBeVisible();
    expect(confirm).toHaveBeenCalledTimes(3);
  });

  it("blocks route leave on a dirty draft and supports both reset and proceed", async () => {
    const user = userEvent.setup();
    const confirm = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    const router = renderMemoryPage();
    await selectClaudeLongTerm(user);
    await user.type(
      screen.getByRole("textbox", { name: "记忆标题" }),
      "未保存",
    );

    act(() => {
      void router.navigate("/other");
    });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/memory");
    });
    expect(screen.getByTestId("memory-page")).toBeVisible();

    act(() => {
      void router.navigate("/other");
    });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/other");
    });
    expect(screen.getByText("其他页面")).toBeVisible();
    expect(confirm).toHaveBeenCalledTimes(2);
  });

  it("leaves the route without confirmation after the current revision is saved", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm");
    const router = renderMemoryPage();
    await selectClaudeLongTerm(user);
    await user.type(
      screen.getByRole("textbox", { name: "记忆标题" }),
      "已保存",
    );
    await user.click(screen.getByRole("button", { name: "保存" }));

    act(() => {
      void router.navigate("/other");
    });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/other");
    });
    expect(confirm).not.toHaveBeenCalled();
  });
});
