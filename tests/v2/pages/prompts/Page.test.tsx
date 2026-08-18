import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  createMemoryRouter,
  Link,
  RouterProvider,
} from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PromptsPage } from "@/v2/pages/prompts/Page";

function PromptTestRoute() {
  return (
    <>
      <PromptsPage />
      <Link to="/memory">离开提示词</Link>
    </>
  );
}

function renderPrompts() {
  const router = createMemoryRouter(
    [
      { path: "/prompts", element: <PromptTestRoute /> },
      { path: "/memory", element: <h1>记忆目标页</h1> },
    ],
    { initialEntries: ["/prompts"] },
  );

  render(<RouterProvider router={router} />);
  return router;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("PromptsPage local-agent prototype", () => {
  it("renders nine grounded rules, canonical resources, and truthful prototype copy", () => {
    renderPrompts();

    expect(screen.getByRole("heading", { name: "提示词" })).toBeVisible();
    expect(screen.getByText("在前端预览中组合长期规则")).toBeVisible();
    expect(
      screen.getByText("前端原型 · 未读取或写入本机文件"),
    ).toBeVisible();
    expect(screen.queryByText("同一应用仅启用一条")).not.toBeInTheDocument();
    expect(screen.getByText("2 条已启用")).toBeVisible();
    expect(screen.getByTestId("prompt-library")).toBeVisible();
    expect(screen.getByTestId("prompt-editor")).toBeVisible();
    expect(screen.getByTestId("prompt-inspector")).toBeVisible();

    const promptList = screen.getByRole("list", { name: "提示词列表" });
    expect(within(promptList).getAllByRole("listitem")).toHaveLength(9);
    expect(screen.getAllByRole("checkbox")).toHaveLength(7);
    expect(screen.getAllByText("启用时创建")).toHaveLength(2);
    expect(screen.getByText("7 个目标文件")).toBeVisible();
    expect(screen.getByText("8 个 Agent")).toBeVisible();
    expect(
      screen.getByText(
        "接入真实同步后，同一路径只执行一次，并保护托管区块外内容",
      ),
    ).toBeVisible();
    expect(screen.getByTestId("prompts-page")).toHaveAttribute(
      "data-data-source",
      "prototype",
    );
  });

  it("allows several rule bundles to stay enabled at the same time", async () => {
    const user = userEvent.setup();
    renderPrompts();

    expect(
      screen.getByRole("switch", { name: "停用中文与回复风格" }),
    ).toBeChecked();
    expect(
      screen.getByRole("switch", { name: "停用目标、边界与完成证据" }),
    ).toBeChecked();

    await user.click(screen.getByRole("switch", { name: "启用代码审查" }));

    expect(screen.getByRole("switch", { name: "停用代码审查" })).toBeChecked();
    expect(
      screen.getByRole("switch", { name: "停用中文与回复风格" }),
    ).toBeChecked();
    expect(
      screen.getByRole("switch", { name: "停用目标、边界与完成证据" }),
    ).toBeChecked();
    expect(screen.getByText("3 条已启用")).toBeVisible();
  });

  it("toggles a different saved rule without disturbing the current dirty editor", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    renderPrompts();

    const content = screen.getByRole<HTMLTextAreaElement>("textbox", {
      name: "内容",
    });
    await user.type(content, "\n未保存的当前规则补充");
    await user.click(screen.getByRole("switch", { name: "启用代码审查" }));

    expect(confirm).not.toHaveBeenCalled();
    expect(screen.getByRole("switch", { name: "停用代码审查" })).toBeChecked();
    expect(content.value).toContain("未保存的当前规则补充");
    expect(
      screen.getByRole("button", {
        name: /^中文与回复风格/,
        pressed: true,
      }),
    ).toBeVisible();
    expect(screen.getByText("3 条已启用")).toBeVisible();
  });

  it("keeps an unsaved new prompt while toggling a different saved rule", async () => {
    const user = userEvent.setup();
    renderPrompts();

    await user.click(screen.getByRole("button", { name: "新建提示词" }));
    const name = screen.getByRole("textbox", { name: "名称" });
    await user.clear(name);
    await user.type(name, "仍可保存的新规则");
    await user.click(screen.getByRole("switch", { name: "启用代码审查" }));

    expect(
      screen.getByRole("button", { name: /^未命名提示词/, pressed: true }),
    ).toBeVisible();
    expect(name).toHaveValue("仍可保存的新规则");
    await user.click(screen.getByRole("checkbox", { name: "注入到Codex全局" }));
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(screen.getByRole("button", { name: /^仍可保存的新规则/ })).toBeVisible();
    expect(
      screen.getByRole("switch", { name: "启用仍可保存的新规则" }),
    ).toBeVisible();
  });

  it("saves target sets and reports canonical file and instance counts", async () => {
    const user = userEvent.setup();
    renderPrompts();

    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    const openClawDefault = screen.getByRole("checkbox", {
      name: "注入到OpenClaw默认工作区 · main + utility",
    });
    expect(openClawDefault).not.toBeChecked();

    await user.click(openClawDefault);
    expect(screen.getByText("5 个目标文件")).toBeVisible();
    expect(screen.getByText("6 个 Agent")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(
      screen.getByText("已保存到前端预览；未写入本机文件"),
    ).toBeVisible();
    expect(
      screen.getByRole("checkbox", {
        name: "取消注入到OpenClaw默认工作区 · main + utility",
      }),
    ).toBeChecked();

    await user.click(
      screen.getByRole("button", { name: /^中文与回复风格/ }),
    );
    expect(screen.getByText("7 个目标文件")).toBeVisible();
    expect(screen.getByText("8 个 Agent")).toBeVisible();

    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    expect(
      screen.getByRole("checkbox", {
        name: "取消注入到OpenClaw默认工作区 · main + utility",
      }),
    ).toBeChecked();
    expect(screen.getByText("5 个目标文件")).toBeVisible();
    expect(screen.getByText("6 个 Agent")).toBeVisible();

    await user.type(
      screen.getByRole("searchbox", { name: "搜索提示词" }),
      "心跳",
    );
    const promptList = screen.getByRole("list", { name: "提示词列表" });
    expect(within(promptList).getByText("定时任务与心跳边界")).toBeVisible();
    expect(
      within(promptList).queryByText("中文与回复风格"),
    ).not.toBeInTheDocument();
  });

  it("requires a transient prompt to be saved before it can be enabled", async () => {
    const user = userEvent.setup();
    renderPrompts();

    await user.click(screen.getByRole("button", { name: "新建提示词" }));
    await user.click(screen.getByRole("switch", { name: "启用未命名提示词" }));
    expect(screen.getByText("请先保存当前提示词，再启用")).toBeVisible();

    await user.click(screen.getByRole("checkbox", { name: "注入到Codex全局" }));
    await user.click(screen.getByRole("switch", { name: "启用未命名提示词" }));
    expect(screen.getByText("请先保存当前提示词，再启用")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "保存" }));
    await user.click(screen.getByRole("switch", { name: "启用未命名提示词" }));
    expect(
      screen.getByRole("switch", { name: "停用未命名提示词" }),
    ).toBeChecked();
  });

  it("uses the last saved targets when validating enable", async () => {
    const user = userEvent.setup();
    renderPrompts();

    await user.click(screen.getByRole("button", { name: "新建提示词" }));
    await user.click(screen.getByRole("button", { name: "保存" }));
    await user.click(screen.getByRole("switch", { name: "启用未命名提示词" }));
    expect(
      screen.getByText("请先选择并保存至少一个注入目标"),
    ).toBeVisible();

    const codexTarget = screen.getByRole("checkbox", {
      name: "注入到Codex全局",
    });
    await user.click(codexTarget);
    await user.click(screen.getByRole("switch", { name: "启用未命名提示词" }));

    expect(codexTarget).toBeChecked();
    expect(
      screen.getByRole("switch", { name: "启用未命名提示词" }),
    ).not.toBeChecked();
    expect(
      screen.getByText("请先选择并保存至少一个注入目标"),
    ).toBeVisible();
  });

  it("keeps the final target on an enabled rule", async () => {
    const user = userEvent.setup();
    renderPrompts();

    await user.click(
      screen.getByRole("button", { name: /^定时任务与心跳边界/ }),
    );
    await user.click(
      screen.getByRole("checkbox", {
        name: "取消注入到OpenClaw群聊工作区 · group_liaison",
      }),
    );
    await user.click(screen.getByRole("button", { name: "保存" }));
    await user.click(
      screen.getByRole("switch", { name: "启用定时任务与心跳边界" }),
    );

    const finalTarget = screen.getByRole("checkbox", {
      name: "取消注入到OpenClaw默认工作区 · main + utility",
    });
    await user.click(finalTarget);

    expect(finalTarget).toBeChecked();
    expect(
      screen.getByText("已启用规则至少保留一个目标；请先停用再清空范围"),
    ).toBeVisible();
  });

  it("keeps an existing-rule draft on cancel and restores its saved baseline after discard", async () => {
    const user = userEvent.setup();
    const confirm = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    renderPrompts();

    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    const description = screen.getByRole("textbox", { name: "描述" });
    const openClawDefault = screen.getByRole("checkbox", {
      name: "注入到OpenClaw默认工作区 · main + utility",
    });

    await user.type(description, " · 尚未保存");
    await user.click(openClawDefault);
    await user.click(
      screen.getByRole("button", { name: /^中文与回复风格/ }),
    );

    expect(confirm).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("textbox", { name: "名称" })).toHaveValue(
      "代码审查",
    );
    expect(description).toHaveValue(
      "优先发现正确性、回归与数据风险 · 尚未保存",
    );
    expect(openClawDefault).toBeChecked();

    await user.click(
      screen.getByRole("button", { name: /^中文与回复风格/ }),
    );
    expect(confirm).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("textbox", { name: "名称" })).toHaveValue(
      "中文与回复风格",
    );

    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    expect(screen.getByRole("textbox", { name: "描述" })).toHaveValue(
      "优先发现正确性、回归与数据风险",
    );
    expect(
      screen.getByRole("checkbox", {
        name: "注入到OpenClaw默认工作区 · main + utility",
      }),
    ).not.toBeChecked();
    expect(screen.getByText("4 个目标文件")).toBeVisible();
    expect(screen.getByText("4 个 Agent")).toBeVisible();
  });

  it("commits a clean enabled toggle without creating route dirtiness", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm");
    renderPrompts();

    await user.click(
      screen.getByRole("switch", { name: "停用中文与回复风格" }),
    );
    await user.click(screen.getByRole("link", { name: "离开提示词" }));

    expect(
      await screen.findByRole("heading", { name: "记忆目标页" }),
    ).toBeVisible();
    expect(confirm).not.toHaveBeenCalled();
  });

  it("preserves dirty fields across a toggle and preserves the committed toggle on discard", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    renderPrompts();

    const openCodeTarget = screen.getByRole("checkbox", {
      name: "取消注入到OpenCode全局",
    });
    await user.type(
      screen.getByRole("textbox", { name: "描述" }),
      " · 尚未保存",
    );
    await user.click(openCodeTarget);
    await user.click(
      screen.getByRole("switch", { name: "停用中文与回复风格" }),
    );

    expect(screen.getByRole("textbox", { name: "描述" })).toHaveValue(
      "中文优先、先说结论、减少空话 · 尚未保存",
    );
    expect(openCodeTarget).not.toBeChecked();
    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    await user.click(screen.getByRole("button", { name: /^中文与回复风格/ }));

    expect(confirm).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("textbox", { name: "描述" })).toHaveValue(
      "中文优先、先说结论、减少空话",
    );
    expect(
      screen.getByRole("switch", { name: "启用中文与回复风格" }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", {
        name: "取消注入到OpenCode全局",
      }),
    ).toBeChecked();
  });

  it("treats a new row as dirty and removes it when discard is confirmed", async () => {
    const user = userEvent.setup();
    const confirm = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    renderPrompts();

    await user.click(screen.getByRole("button", { name: "新建提示词" }));
    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    expect(screen.getByRole("textbox", { name: "名称" })).toHaveValue(
      "未命名提示词",
    );

    await user.click(screen.getByRole("button", { name: /^代码审查/ }));
    expect(screen.getByRole("textbox", { name: "名称" })).toHaveValue("代码审查");
    expect(
      screen.queryByRole("switch", { name: "启用未命名提示词" }),
    ).not.toBeInTheDocument();
    expect(confirm).toHaveBeenCalledTimes(2);
  });

  it("cancels and then proceeds through a dirty pathname change", async () => {
    const user = userEvent.setup();
    const confirm = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);
    const router = renderPrompts();

    await user.type(screen.getByRole("textbox", { name: "描述" }), "尚未保存");
    await user.click(screen.getByRole("link", { name: "离开提示词" }));

    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(router.state.location.pathname).toBe("/prompts");
    expect(screen.getByRole("heading", { name: "提示词" })).toBeVisible();

    await user.click(screen.getByRole("link", { name: "离开提示词" }));
    expect(
      await screen.findByRole("heading", { name: "记忆目标页" }),
    ).toBeVisible();
    expect(confirm).toHaveBeenCalledTimes(2);
  });

  it("leaves without confirmation after saving a dirty draft", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm");
    renderPrompts();

    await user.type(screen.getByRole("textbox", { name: "描述" }), "已确认");
    await user.click(screen.getByRole("button", { name: "保存" }));
    await user.click(screen.getByRole("link", { name: "离开提示词" }));

    expect(
      await screen.findByRole("heading", { name: "记忆目标页" }),
    ).toBeVisible();
    expect(confirm).not.toHaveBeenCalled();
  });
});
