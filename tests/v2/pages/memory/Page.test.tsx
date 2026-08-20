import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { MemoryPage } from "@/v2/pages/memory/Page";
import type { FeaturePorts } from "@/v2/shared/features/ports";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { PrimaryBlockerProvider } from "@/v2/shared/ui/PrimaryBlocker";
import type {
  DailyMemoryFileInfo,
  DailyMemorySearchResult,
  HermesMemoryLimits,
  MemoryDocumentId,
} from "@/v2/shared/features/types";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";

const DOCUMENT_IDS: readonly MemoryDocumentId[] = [
  "openclaw-memory",
  "openclaw-user",
  "hermes-memory",
  "hermes-user",
];

type MemoryStores = {
  documents: Record<MemoryDocumentId, string | null>;
  daily: Record<string, string>;
  limits: HermesMemoryLimits;
};

function statefulMemoryPorts(
  initialDocuments: Partial<Record<MemoryDocumentId, string | null>> = {},
  initialDaily: Record<string, string> = {},
) {
  const stores: MemoryStores = {
    documents: {
      "openclaw-memory": initialDocuments["openclaw-memory"] ?? null,
      "openclaw-user": initialDocuments["openclaw-user"] ?? null,
      "hermes-memory": initialDocuments["hermes-memory"] ?? "",
      "hermes-user": initialDocuments["hermes-user"] ?? "",
    },
    daily: { ...initialDaily },
    limits: {
      memory: 4_000,
      user: 2_000,
      memoryEnabled: true,
      userEnabled: false,
    },
  };
  const ports = createBrowserFeaturePorts();

  ports.memory.readDocument = vi.fn(
    async (id: MemoryDocumentId) => stores.documents[id],
  );
  ports.memory.writeDocument = vi.fn(
    async (id: MemoryDocumentId, content: string) => {
      stores.documents[id] = content;
    },
  );
  ports.memory.getHermesLimits = vi.fn(async () => ({ ...stores.limits }));
  ports.memory.setHermesEnabled = vi.fn(async (kind, enabled) => {
    if (kind === "memory") stores.limits.memoryEnabled = enabled;
    else stores.limits.userEnabled = enabled;
  });
  ports.memory.listDailyFiles = vi.fn(
    async (): Promise<DailyMemoryFileInfo[]> =>
      Object.entries(stores.daily)
        .sort(([left], [right]) => right.localeCompare(left))
        .map(([filename, content]) => ({
          filename,
          date: filename.slice(0, 10),
          sizeBytes: content.length,
          modifiedAt: 1_700_000_000_000,
          preview: content.slice(0, 40),
        })),
  );
  ports.memory.readDailyFile = vi.fn(async (filename: string) =>
    Object.prototype.hasOwnProperty.call(stores.daily, filename)
      ? stores.daily[filename]
      : null,
  );
  ports.memory.writeDailyFile = vi.fn(
    async (filename: string, content: string) => {
      stores.daily[filename] = content;
    },
  );
  ports.memory.deleteDailyFile = vi.fn(async (filename: string) => {
    delete stores.daily[filename];
  });
  ports.memory.searchDailyFiles = vi.fn(
    async (query: string): Promise<DailyMemorySearchResult[]> =>
      Object.entries(stores.daily)
        .filter(([, content]) => content.includes(query))
        .map(([filename, content]) => ({
          filename,
          date: filename.slice(0, 10),
          sizeBytes: content.length,
          modifiedAt: 1_700_000_000_000,
          snippet: content,
          matchCount: content.split(query).length - 1,
        })),
  );
  ports.memory.openOpenClawDirectory = vi.fn(async () => undefined);

  return { ports, stores };
}

function renderMemory(ports: FeaturePorts) {
  const router = createMemoryRouter(
    [
      {
        path: "/memory",
        element: (
          <PrimaryBlockerProvider>
            <MemoryPage />
          </PrimaryBlockerProvider>
        ),
      },
      { path: "/other", element: <main>其他页面</main> },
    ],
    { initialEntries: ["/memory"] },
  );
  render(
    <FeatureProvider ports={ports}>
      <RouterProvider router={router} />
    </FeatureProvider>,
  );
  return router;
}

function localTodayFilename(): string {
  const today = new Date();
  return `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(
    2,
    "0",
  )}-${String(today.getDate()).padStart(2, "0")}.md`;
}

async function confirmDialog(
  user: ReturnType<typeof userEvent.setup>,
  title: string | RegExp,
) {
  const dialog = await screen.findByRole("dialog", { name: title });
  await user.click(within(dialog).getByRole("button", { name: "确认" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("dialog", { name: title }),
    ).not.toBeInTheDocument(),
  );
  await act(async () => {
    await new Promise((resolve) => window.setTimeout(resolve, 0));
  });
}

async function cancelDialog(
  user: ReturnType<typeof userEvent.setup>,
  title: string | RegExp,
) {
  const dialog = await screen.findByRole("dialog", { name: title });
  await user.click(within(dialog).getByRole("button", { name: "取消" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("dialog", { name: title }),
    ).not.toBeInTheDocument(),
  );
  await act(async () => {
    await new Promise((resolve) => window.setTimeout(resolve, 0));
  });
}

describe("MemoryPage native business management", () => {
  it("loads and switches among exactly four fixed long-term resources", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "openclaw memory",
      "openclaw-user": "openclaw user",
      "hermes-memory": "hermes memory",
      "hermes-user": "hermes user",
    });
    const user = userEvent.setup();
    renderMemory(ports);

    const resources = await screen.findByRole("region", {
      name: "长期记忆资源",
    });
    expect(within(resources).getAllByRole("button")).toHaveLength(4);
    expect(within(resources).getAllByText("MEMORY.md")).toHaveLength(2);
    expect(within(resources).getAllByText("USER.md")).toHaveLength(2);
    expect(screen.getByRole("textbox", { name: "记忆内容" })).toHaveValue(
      "openclaw memory",
    );
    expect(screen.getByRole("textbox", { name: "记忆内容" })).toBeVisible();
    expect(screen.queryByText("记忆信息")).not.toBeInTheDocument();
    expect(screen.queryByText("使用说明")).not.toBeInTheDocument();

    const selections: ReadonlyArray<[RegExp, MemoryDocumentId, string]> = [
      [/OpenClaw · USER\.md/, "openclaw-user", "openclaw user"],
      [/Hermes · MEMORY\.md/, "hermes-memory", "hermes memory"],
      [/Hermes · USER\.md/, "hermes-user", "hermes user"],
    ];
    for (const [name, id, content] of selections) {
      const currentResources = await screen.findByRole("region", {
        name: "长期记忆资源",
      });
      await user.click(within(currentResources).getByRole("button", { name }));
      await waitFor(() =>
        expect(screen.getByRole("textbox", { name: "记忆内容" })).toHaveValue(
          content,
        ),
      );
      expect(ports.memory.readDocument).toHaveBeenCalledWith(id);
    }
    expect(ports.memory.readDocument).toHaveBeenCalledWith(DOCUMENT_IDS[0]);
    expect(document.body.textContent).not.toMatch(
      /前端原型|会话记录|重新扫描本机|同步预览|提炼草稿|修订 r\d+|待执行同步任务/,
    );
  });

  it("keeps a missing OpenClaw document absent until explicit save", async () => {
    const { ports, stores } = statefulMemoryPorts({
      "openclaw-memory": null,
    });
    const user = userEvent.setup();
    renderMemory(ports);

    expect(
      await screen.findByText("此内容尚未创建。点击“保存”后即可创建。"),
    ).toBeVisible();
    expect(ports.memory.writeDocument).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox", { name: "记忆内容" })).toHaveValue("");

    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(ports.memory.writeDocument).toHaveBeenCalledWith(
        "openclaw-memory",
        "",
      ),
    );
    expect(stores.documents["openclaw-memory"]).toBe("");
    expect(
      await screen.findByText("OpenClaw · MEMORY.md 已保存"),
    ).toBeVisible();
    expect(
      screen.queryByText("此内容尚未创建。点击“保存”后即可创建。"),
    ).not.toBeInTheDocument();
  });

  it("shows real Hermes content, limits, independent toggle state, and a saveable over-limit warning", async () => {
    const { ports, stores } = statefulMemoryPorts({
      "openclaw-memory": "OpenClaw",
      "hermes-memory": "123456",
    });
    stores.limits.memory = 5;
    stores.limits.memoryEnabled = true;
    const user = userEvent.setup();
    renderMemory(ports);
    const resources = await screen.findByRole("region", {
      name: "长期记忆资源",
    });

    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
    await user.click(
      within(resources).getByRole("button", { name: /Hermes · MEMORY\.md/ }),
    );
    expect(
      await screen.findByRole("textbox", { name: "记忆内容" }),
    ).toHaveValue("123456");
    expect(
      within(screen.getByRole("region", { name: "长期记忆编辑器" })).getByRole(
        "status",
      ),
    ).toHaveTextContent(/超过 Hermes 的\s*5\s*字符上限/);
    expect(screen.getByText(/6 \/ 5/)).toBeVisible();

    const toggle = screen.getByRole("switch", {
      name: "在 Hermes 中停用 Hermes · MEMORY.md",
    });
    expect(toggle).toBeChecked();
    await user.click(toggle);
    await waitFor(() =>
      expect(ports.memory.setHermesEnabled).toHaveBeenCalledWith(
        "memory",
        false,
      ),
    );
    expect(
      await screen.findByRole("switch", {
        name: "在 Hermes 中启用 Hermes · MEMORY.md",
      }),
    ).not.toBeChecked();

    await user.type(screen.getByRole("textbox", { name: "记忆内容" }), "7");
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(stores.documents["hermes-memory"]).toBe("1234567"),
    );
    expect(await screen.findByText("Hermes · MEMORY.md 已保存")).toBeVisible();

    await user.click(
      within(
        await screen.findByRole("region", { name: "长期记忆资源" }),
      ).getByRole("button", { name: /Hermes · USER\.md/ }),
    );
    const userToggle = await screen.findByRole("switch", {
      name: "在 Hermes 中启用 Hermes · USER.md",
    });
    expect(userToggle).not.toBeChecked();
    await user.click(userToggle);
    await waitFor(() =>
      expect(ports.memory.setHermesEnabled).toHaveBeenCalledWith("user", true),
    );
    expect(stores.limits.memoryEnabled).toBe(false);
    expect(stores.limits.userEnabled).toBe(true);
  });

  it("uses the authoritative long-term reread after save", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "baseline",
    });
    ports.memory.writeDocument = vi.fn(async () => undefined);
    let reads = 0;
    ports.memory.readDocument = vi.fn(async () => {
      reads += 1;
      return reads === 1 ? "baseline" : "native normalized";
    });
    const user = userEvent.setup();
    renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });

    await user.clear(editor);
    await user.type(editor, "local draft");
    await user.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => expect(editor).toHaveValue("native normalized"));
    expect(ports.memory.readDocument).toHaveBeenCalledTimes(2);
    expect(screen.getByText("OpenClaw · MEMORY.md 已保存")).toBeVisible();
  });

  it("reports a write failure without replacing the dirty long-term draft", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "baseline",
    });
    ports.memory.writeDocument = vi.fn(async () => {
      throw new Error("save rejected");
    });
    const user = userEvent.setup();
    renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });

    await user.type(editor, " dirty");
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(
      await screen.findByText("保存长期记忆失败：请稍后重试。"),
    ).toBeVisible();
    expect(screen.queryByText("save rejected")).not.toBeInTheDocument();
    expect(editor).toHaveValue("baseline dirty");
    expect(screen.getByText("未保存")).toBeVisible();
    expect(
      screen.queryByText("OpenClaw · MEMORY.md 已保存"),
    ).not.toBeInTheDocument();
  });

  it("retains the prior baseline and warns when the post-write refresh fails", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "baseline",
    });
    let reads = 0;
    ports.memory.readDocument = vi.fn(async () => {
      reads += 1;
      if (reads === 1) return "baseline";
      throw new Error("refresh unavailable");
    });
    const user = userEvent.setup();
    renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });

    await user.type(editor, " dirty");
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(
      await screen.findByText(
        "写入可能已完成，但状态刷新失败：请稍后重试。",
        undefined,
        { timeout: 5_000 },
      ),
    ).toBeVisible();
    expect(editor).toHaveValue("baseline dirty");
    expect(screen.getByText("未保存")).toBeVisible();
    expect(
      screen.queryByText("OpenClaw · MEMORY.md 已保存"),
    ).not.toBeInTheDocument();
  });

  it("locks duplicate long-term saves until the write and reread finish", async () => {
    let resolveWrite!: () => void;
    const pendingWrite = new Promise<void>((resolve) => {
      resolveWrite = resolve;
    });
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "baseline",
    });
    ports.memory.writeDocument = vi.fn(() => pendingWrite);
    const user = userEvent.setup();
    renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });

    await user.type(editor, " dirty");
    await user.click(screen.getByRole("button", { name: "保存" }));
    const pending = screen.getByRole("button", { name: "保存中…" });
    expect(pending).toBeDisabled();
    fireEvent.click(pending);
    expect(ports.memory.writeDocument).toHaveBeenCalledTimes(1);

    resolveWrite();
    expect(
      await screen.findByText("OpenClaw · MEMORY.md 已保存"),
    ).toBeVisible();
  });

  it("lists, reads, debounced-searches, creates, saves, deletes, and opens daily memory", async () => {
    const today = localTodayFilename();
    const { ports, stores } = statefulMemoryPorts(
      { "openclaw-memory": "baseline" },
      {
        "2026-08-13.md": "ordinary daily note",
        "2026-08-12.md": "needle appears twice: needle",
      },
    );
    const user = userEvent.setup();
    renderMemory(ports);
    await screen.findByRole("textbox", { name: "记忆内容" });
    await user.click(screen.getByRole("tab", { name: "每日记忆" }));

    expect(
      await screen.findByRole("textbox", { name: "每日记忆内容" }),
    ).toHaveValue("ordinary daily note");
    expect(ports.memory.readDailyFile).toHaveBeenCalledWith("2026-08-13.md");
    await user.click(screen.getByRole("button", { name: "打开记忆目录" }));
    expect(ports.memory.openOpenClawDirectory).toHaveBeenCalledWith("memory");

    const search = screen.getByRole("searchbox", { name: "搜索每日记忆" });
    await user.type(search, "needle");
    expect(ports.memory.searchDailyFiles).not.toHaveBeenCalled();
    await new Promise((resolve) => window.setTimeout(resolve, 150));
    expect(ports.memory.searchDailyFiles).not.toHaveBeenCalled();
    await waitFor(
      () =>
        expect(ports.memory.searchDailyFiles).toHaveBeenCalledWith("needle"),
      { timeout: 1_500 },
    );
    expect(await screen.findByText(/2 处匹配/)).toBeVisible();
    await user.clear(search);
    await user.type(search, "no matching memory");
    expect(
      await screen.findByText("没有匹配的每日记忆", undefined, {
        timeout: 1_500,
      }),
    ).toBeVisible();
    await user.clear(search);

    await user.click(screen.getByRole("button", { name: "创建或打开今天" }));
    expect(await screen.findByRole("heading", { name: today })).toBeVisible();
    expect(
      screen.getByText("今天的记录尚未创建。点击“保存”后即可创建。"),
    ).toBeVisible();
    expect(ports.memory.writeDailyFile).not.toHaveBeenCalled();

    const editor = screen.getByRole("textbox", { name: "每日记忆内容" });
    await user.type(editor, "today content");
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(ports.memory.writeDailyFile).toHaveBeenCalledWith(
        today,
        "today content",
      ),
    );
    expect(stores.daily[today]).toBe("today content");
    expect(await screen.findByText(`${today} 已保存`)).toBeVisible();

    await user.click(screen.getByRole("button", { name: "删除" }));
    await confirmDialog(user, `删除 ${today}？`);
    await waitFor(() =>
      expect(ports.memory.deleteDailyFile).toHaveBeenCalledWith(today),
    );
    expect(stores.daily[today]).toBeUndefined();
    expect(await screen.findByText(`${today} 已删除`)).toBeVisible();
  });

  it("guards dirty long-term document and tab changes with the shared dialog", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "memory",
      "openclaw-user": "user",
    });
    const user = userEvent.setup();
    renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });
    await user.type(editor, " dirty");

    let resources = screen.getByRole("region", { name: "长期记忆资源" });
    await user.click(
      within(resources).getByRole("button", { name: /OpenClaw · USER\.md/ }),
    );
    await cancelDialog(user, "放弃未保存的更改？");
    expect(editor).toHaveValue("memory dirty");

    resources = screen.getByRole("region", { name: "长期记忆资源" });
    await user.click(
      within(resources).getByRole("button", { name: /OpenClaw · USER\.md/ }),
    );
    await confirmDialog(user, "放弃未保存的更改？");
    const userEditor = await screen.findByRole("textbox", { name: "记忆内容" });
    expect(userEditor).toHaveValue("user");

    await user.type(userEditor, " dirty");
    await user.click(screen.getByRole("tab", { name: "每日记忆" }));
    await cancelDialog(user, "放弃未保存的更改？");
    expect(
      screen.getByRole("tab", { name: "长期记忆", selected: true }),
    ).toBeVisible();
    await user.click(screen.getByRole("tab", { name: "每日记忆" }));
    await confirmDialog(user, "放弃未保存的更改？");
    expect(
      await screen.findByRole("tab", { name: "每日记忆", selected: true }),
    ).toBeVisible();
  });

  it("guards dirty daily file changes", async () => {
    const { ports } = statefulMemoryPorts(
      { "openclaw-memory": "memory" },
      {
        "2026-08-13.md": "newer",
        "2026-08-12.md": "older",
      },
    );
    const user = userEvent.setup();
    renderMemory(ports);
    await screen.findByRole("textbox", { name: "记忆内容" });
    await user.click(screen.getByRole("tab", { name: "每日记忆" }));
    let editor = await screen.findByRole("textbox", { name: "每日记忆内容" });
    await user.type(editor, " dirty");

    const list = screen.getByRole("region", { name: "每日记忆列表" });
    await user.click(
      within(list).getByRole("button", { name: /2026-08-12\.md/ }),
    );
    await cancelDialog(user, "放弃未保存的更改？");
    expect(editor).toHaveValue("newer dirty");

    await user.click(
      within(screen.getByRole("region", { name: "每日记忆列表" })).getByRole(
        "button",
        { name: /2026-08-12\.md/ },
      ),
    );
    await confirmDialog(user, "放弃未保存的更改？");
    editor = await screen.findByRole("textbox", { name: "每日记忆内容" });
    expect(editor).toHaveValue("older");
  });

  it("cancels dirty route navigation with the shared dialog", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "memory",
    });
    const user = userEvent.setup();
    const router = renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });
    await user.type(editor, " dirty");

    await act(async () => {
      await router.navigate("/other");
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    });
    const routeDialog = await screen.findByRole("dialog", {
      name: "放弃未保存的更改？",
    });
    await user.click(within(routeDialog).getByRole("button", { name: "取消" }));
    await waitFor(() => expect(routeDialog).not.toBeInTheDocument());
    expect(router.state.location.pathname).toBe("/memory");
  });

  it("confirms dirty route navigation with the shared dialog", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "memory",
    });
    const user = userEvent.setup();
    const router = renderMemory(ports);
    const editor = await screen.findByRole("textbox", { name: "记忆内容" });
    await user.type(editor, " dirty");

    await act(async () => {
      await router.navigate("/other");
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    });
    const routeDialog = await screen.findByRole("dialog", {
      name: "放弃未保存的更改？",
    });
    await user.click(within(routeDialog).getByRole("button", { name: "确认" }));
    expect(await screen.findByText("其他页面")).toBeVisible();
    expect(router.state.location.pathname).toBe("/other");
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    });
  });

  it("renders a truthful browser native-only state without seeded memory", async () => {
    renderMemory(createBrowserFeaturePorts());

    expect(
      await screen.findByText("需要 FyAgent 桌面应用", undefined, {
        timeout: 5_000,
      }),
    ).toBeVisible();
    expect(document.body.textContent).not.toMatch(
      /示例会话|模拟扫描|同步任务|提炼草稿|Codex · 记忆/,
    );
  });

  it("shows long-term loading before rendering authoritative content", async () => {
    let resolveRead!: (content: string | null) => void;
    const pendingRead = new Promise<string | null>((resolve) => {
      resolveRead = resolve;
    });
    const { ports } = statefulMemoryPorts();
    ports.memory.readDocument = vi.fn(() => pendingRead);
    renderMemory(ports);

    expect(screen.getByText("正在加载长期记忆")).toBeVisible();
    resolveRead("loaded");
    expect(
      await screen.findByRole("textbox", { name: "记忆内容" }),
    ).toHaveValue("loaded");
  });

  it("distinguishes a real long-term read error from native-only mode", async () => {
    const { ports } = statefulMemoryPorts();
    ports.memory.readDocument = vi.fn(async () => {
      throw new Error("workspace unreadable");
    });
    renderMemory(ports);

    expect(
      await screen.findByText("无法加载长期记忆", undefined, {
        timeout: 5_000,
      }),
    ).toBeVisible();
    expect(screen.getByText("请稍后重试。")).toBeVisible();
    expect(screen.queryByText("workspace unreadable")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeVisible();
    expect(screen.queryByText("需要 FyAgent 桌面应用")).not.toBeInTheDocument();
  });

  it("distinguishes real daily empty data and a daily list error", async () => {
    const { ports } = statefulMemoryPorts({
      "openclaw-memory": "memory",
    });
    ports.memory.listDailyFiles = vi.fn(async () => []);
    const user = userEvent.setup();
    renderMemory(ports);
    await screen.findByRole("textbox", { name: "记忆内容" });
    await user.click(screen.getByRole("tab", { name: "每日记忆" }));
    expect(await screen.findByText("还没有每日记忆")).toBeVisible();
    expect(
      screen.getAllByRole("button", { name: "创建或打开今天" }),
    ).toHaveLength(2);
  });

  it("shows native operation errors for daily search and directory open", async () => {
    const { ports } = statefulMemoryPorts(
      { "openclaw-memory": "memory" },
      { "2026-08-13.md": "daily" },
    );
    ports.memory.searchDailyFiles = vi.fn(async () => {
      throw new Error("search unavailable");
    });
    ports.memory.openOpenClawDirectory = vi.fn(async () => {
      throw new Error("open rejected");
    });
    const user = userEvent.setup();
    renderMemory(ports);
    await screen.findByRole("textbox", { name: "记忆内容" });
    await user.click(screen.getByRole("tab", { name: "每日记忆" }));
    await screen.findByRole("textbox", { name: "每日记忆内容" });

    await user.click(screen.getByRole("button", { name: "打开记忆目录" }));
    expect(await screen.findByText("无法打开目录：请稍后重试。")).toBeVisible();
    expect(screen.queryByText("open rejected")).not.toBeInTheDocument();
    await user.type(
      screen.getByRole("searchbox", { name: "搜索每日记忆" }),
      "missing",
    );
    expect(
      await screen.findByText("搜索失败：请稍后重试。", undefined, {
        timeout: 5_000,
      }),
    ).toBeVisible();
  });
});
