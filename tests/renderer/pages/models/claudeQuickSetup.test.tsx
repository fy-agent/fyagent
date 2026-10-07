import type { ConfigRecoverySnapshot } from "@/shared/features/config-recovery";
import { useState } from "react";
import { PersistentSurface } from "@/shared/ui/PersistentSurface";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { ModelsPage } from "@/pages/models/Page";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import type {
  ClaudeQuickSetupOutcome,
  ClaudeQuickSetupPreview,
} from "@/shared/features/claude-quick-setup";

const previewId = "11111111-1111-4111-8111-111111111111";
const preview: ClaudeQuickSetupPreview = {
  contractVersion: 1,
  previewId,
  writeTargets: [
    {
      path: "~/.claude/settings.json",
      backupPath: "~/.claude/settings.json.fyagent.backup",
      exists: false,
    },
    {
      path: "~/.claude.json",
      backupPath: "~/.claude.json.fyagent.backup",
      exists: true,
    },
  ],
  preservedPaths: [],
  sidecars: [
    {
      target: "claude_settings",
      backupPath: null,
      undoPath: "~/.claude/settings.json.fyagent.undo.json",
    },
    {
      target: "claude_mcp",
      backupPath: "~/.claude.json.fyagent.backup",
      undoPath: "~/.claude.json.fyagent.undo.json",
    },
  ],
};
const applied: ClaudeQuickSetupOutcome = {
  contractVersion: 1,
  overall: "applied",
  providerState: "applied",
  files: [
    { target: "claude_settings", state: "applied" },
    { target: "claude_mcp", state: "applied" },
  ],
};
function setup() {
  const ports = createBrowserFeaturePorts();
  ports.providers.getSummary = vi.fn(async () => ({
    providers: {},
    currentId: "fyagent-v2-quick-setup-claude",
    writeTargets: [],
  }));
  ports.providers.previewClaudeQuickSetup = vi.fn(async () => preview);
  ports.providers.applyClaudeQuickSetupPreview = vi.fn(async () => applied);
  ports.providers.applyQuickSetupWithResult = vi.fn();
  let setVisible!: (visible: boolean) => void;
  function Host() {
    const [visible, setState] = useState(true);
    setVisible = setState;
    return (
      <MemoryRouter initialEntries={["/models?target=claude"]}>
        <TooltipProvider>
          <FeatureProvider ports={ports}>
            <PersistentSurface active={visible}>
              <ModelsPage />
            </PersistentSurface>
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    );
  }
  const view = render(<Host />);
  return { ports, view, setVisible, user: userEvent.setup() };
}
async function draft(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByRole("heading", { name: "Claude Code" });
  await user.type(
    screen.getByLabelText("服务地址"),
    "https://claude.example.test",
  );
  await user.type(screen.getByLabelText("API Key"), "private-fixture-key");
  await user.type(screen.getByLabelText("模型 ID"), "fixture-model");
}
async function save(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "保存并设为当前配置" }));
}
async function confirm(user: ReturnType<typeof userEvent.setup>) {
  const dialog = await screen.findByRole("dialog", { name: "保存前确认" });
  await user.click(within(dialog).getByRole("button", { name: "确认保存" }));
}
describe("Claude Models native consent", () => {
  it("shows the actual two-file scope, cancels without apply, and confirms identity once", async () => {
    const { ports, user } = setup();
    await draft(user);
    await save(user);
    const dialog = await screen.findByRole("dialog", { name: "保存前确认" });
    expect(within(dialog).getByText("将创建")).toBeVisible();
    expect(within(dialog).getByText("将修改")).toBeVisible();
    expect(
      within(dialog).getByText("~/.claude.json.fyagent.undo.json"),
    ).toBeVisible();
    expect(dialog).not.toHaveTextContent("private-fixture-key");
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(ports.providers.applyClaudeQuickSetupPreview).not.toHaveBeenCalled();
    expect(screen.getByLabelText("API Key")).toHaveValue("private-fixture-key");
    await save(user);
    const confirmButton = within(
      await screen.findByRole("dialog", { name: "保存前确认" }),
    ).getByRole("button", { name: "确认保存" });
    fireEvent.click(confirmButton);
    fireEvent.click(confirmButton);
    await screen.findByText("模型设置已保存并设为当前配置");
    expect(ports.providers.applyClaudeQuickSetupPreview).toHaveBeenCalledTimes(
      1,
    );
    expect(ports.providers.applyClaudeQuickSetupPreview).toHaveBeenCalledWith({
      previewId,
    });
    expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
  });
  it.each(["edit", "switch", "hide", "unmount"] as const)(
    "discards a late preview after %s",
    async (action) => {
      const { ports, user, view, setVisible } = setup();
      let resolve!: (value: ClaudeQuickSetupPreview) => void;
      ports.providers.previewClaudeQuickSetup = vi.fn(
        () =>
          new Promise<ClaudeQuickSetupPreview>((done) => {
            resolve = done;
          }),
      );
      await draft(user);
      await save(user);
      if (action === "edit")
        fireEvent.change(screen.getByLabelText("模型 ID"), {
          target: { value: "new-model" },
        });
      if (action === "switch")
        await user.click(screen.getByTestId("model-target-codex"));
      if (action === "hide") act(() => setVisible(false));
      if (action === "unmount") view.unmount();
      await act(async () => {
        resolve(preview);
      });
      if (action === "hide") act(() => setVisible(true));
      expect(screen.queryByRole("dialog", { name: "保存前确认" })).toBeNull();
      expect(
        ports.providers.applyClaudeQuickSetupPreview,
      ).not.toHaveBeenCalled();
    },
  );
  it.each(["stale", "rolledBack"] as const)(
    "retains the draft and key after %s and permits a fresh preview",
    async (overall) => {
      const { ports, user } = setup();
      ports.providers.applyClaudeQuickSetupPreview = vi.fn(
        async (): Promise<ClaudeQuickSetupOutcome> => ({
          contractVersion: 1,
          overall,
          providerState: overall === "stale" ? "unchanged" : "rolledBack",
          files: [
            {
              target: "claude_settings",
              state: overall === "stale" ? "notAttempted" : "rolledBack",
            },
            { target: "claude_mcp", state: "unchanged" },
          ],
        }),
      );
      await draft(user);
      await save(user);
      await confirm(user);
      await screen.findByText(
        overall === "stale"
          ? "保存范围已变化，未写入"
          : "未能保存设置，已还原之前的状态",
      );
      expect(screen.getByLabelText("API Key")).toHaveValue(
        "private-fixture-key",
      );
      await waitFor(() =>
        expect(
          screen.getByRole("button", { name: "保存并设为当前配置" }),
        ).toBeEnabled(),
      );
      await save(user);
      expect(ports.providers.previewClaudeQuickSetup).toHaveBeenCalledTimes(2);
    },
  );
  it("does not upgrade a partial result from matching currentId and leaves independent recovery available", async () => {
    const { ports, user } = setup();
    ports.providers.applyClaudeQuickSetupPreview = vi.fn(
      async (): Promise<ClaudeQuickSetupOutcome> => ({
        ...applied,
        overall: "partial",
        files: [applied.files[0], { target: "claude_mcp", state: "conflict" }],
      }),
    );
    await draft(user);
    await save(user);
    await confirm(user);
    await screen.findByText("模型条目已保存，部分文件未完成");
    expect(screen.queryByText("模型设置已保存并设为当前配置")).toBeNull();
    expect(screen.getByText(/\.claude.json：存在外部改动/)).toBeVisible();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "撤回文件修改" }),
      ).toBeEnabled(),
    );
    expect(
      screen.getByRole("button", { name: "暂时无法确认当前设置" }),
    ).toBeDisabled();
  });
  it("releases the write lock when reread fails without upgrading the native result", async () => {
    const { ports, user } = setup();
    await draft(user);
    await save(user);
    ports.providers.getSummary = vi.fn(async () => {
      throw new Error("reread failed");
    });
    await confirm(user);
    await screen.findByText("模型设置已保存并设为当前配置");
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "撤回文件修改" }),
      ).toBeEnabled(),
    );
  });
  it("keeps credentials after a preview failure and permits retry", async () => {
    const { ports, user } = setup();
    ports.providers.previewClaudeQuickSetup = vi
      .fn()
      .mockRejectedValueOnce(new Error("private failure"))
      .mockResolvedValueOnce(preview);
    await draft(user);
    await save(user);
    await screen.findByText("无法读取保存范围");
    expect(screen.getByLabelText("API Key")).toHaveValue("private-fixture-key");
    expect(ports.providers.applyClaudeQuickSetupPreview).not.toHaveBeenCalled();
    await save(user);
    await screen.findByRole("dialog", { name: "保存前确认" });
    expect(ports.providers.previewClaudeQuickSetup).toHaveBeenCalledTimes(2);
  });

  it("keeps unknown writes blocked after restoring only settings and retains the Provider", async () => {
    const { ports, user } = setup();
    ports.providers.getSummary = vi.fn(async () => ({
      providers: {
        "fyagent-v2-quick-setup-claude": {
          id: "fyagent-v2-quick-setup-claude",
          name: "Retained provider",
        },
      },
      currentId: "fyagent-v2-quick-setup-claude",
      writeTargets: [],
    }));
    ports.providers.applyClaudeQuickSetupPreview = vi.fn(
      async (): Promise<ClaudeQuickSetupOutcome> => ({
        ...applied,
        overall: "unknown",
        providerState: "unknown",
        files: [
          { target: "claude_settings", state: "unknown" },
          { target: "claude_mcp", state: "conflict" },
        ],
      }),
    );
    let restored = false;
    const recoveries = (): ConfigRecoverySnapshot[] => [
      {
        contractVersion: 1,
        target: "claude_settings",
        writeTarget: preview.writeTargets[0],
        state: restored ? "none" : "available",
        receiptId: restored ? null : previewId,
        restoresExistingFile: restored ? null : false,
      },
      {
        contractVersion: 1,
        target: "claude_mcp",
        writeTarget: preview.writeTargets[1],
        state: "conflict",
        receiptId: null,
        restoresExistingFile: null,
      },
    ];
    ports.configRecovery.list = vi.fn(async () => recoveries());
    ports.configRecovery.restore = vi.fn(async () => {
      restored = true;
      return recoveries()[0];
    });
    await draft(user);
    await save(user);
    await confirm(user);
    await screen.findByText("无法确认当前设置");
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "撤回文件修改" }),
      ).toBeEnabled(),
    );
    await user.click(screen.getByRole("button", { name: "撤回文件修改" }));
    const dialog = await screen.findByRole("dialog", { name: "撤回文件修改" });
    expect(ports.configRecovery.list).toHaveBeenCalledWith([
      "claude_settings",
      "claude_mcp",
    ]);
    const settings = await within(dialog).findByRole("radio", {
      name: "Claude Code 配置",
    });
    expect(
      within(dialog).getByRole("radio", { name: "Claude Code MCP 配置" }),
    ).toBeDisabled();
    await user.click(settings);
    await user.click(
      within(dialog).getByRole("button", { name: "确认删除新文件" }),
    );
    await screen.findByText("文件已恢复");
    expect(ports.configRecovery.restore).toHaveBeenCalledWith({
      target: "claude_settings",
      receiptId: previewId,
    });
    await user.click(within(dialog).getByRole("button", { name: "关闭" }));
    expect(
      screen.getByRole("button", { name: "暂时无法确认当前设置" }),
    ).toBeDisabled();
    expect(screen.getByText("已有设置，将更新")).toBeVisible();
    expect(ports.providers.applyClaudeQuickSetupPreview).toHaveBeenCalledTimes(
      1,
    );
  });
  it("reports a failed root writer with retained preimage as partial, not unattempted or full rollback", async () => {
    const { ports, user } = setup();
    ports.providers.applyClaudeQuickSetupPreview = vi.fn(
      async (): Promise<ClaudeQuickSetupOutcome> => ({
        ...applied,
        overall: "partial",
        files: [
          applied.files[0],
          { target: "claude_mcp", state: "rolledBack" },
        ],
      }),
    );
    await draft(user);
    await save(user);
    await confirm(user);
    await screen.findByText("模型条目已保存，部分文件未完成");
    expect(
      screen.getByText(/\.claude.json：未完成，已保留或还原此前内容/),
    ).toBeVisible();
    expect(screen.queryByText("未能保存设置，已还原之前的状态")).toBeNull();
    expect(screen.queryByText(/\.claude.json：未写入/)).toBeNull();
  });
  it.each(["unknown", "partial", "reject"] as const)(
    "keeps Claude writes blocked when %s arrives after switching away from the applying panel",
    async (result) => {
      const { ports, user } = setup();
      let resolve!: (outcome: ClaudeQuickSetupOutcome) => void;
      let reject!: (reason: Error) => void;
      ports.providers.applyClaudeQuickSetupPreview = vi.fn(
        () =>
          new Promise<ClaudeQuickSetupOutcome>((done, fail) => {
            resolve = done;
            reject = fail;
          }),
      );
      await draft(user);
      await save(user);
      await confirm(user);
      await waitFor(() =>
        expect(
          ports.providers.applyClaudeQuickSetupPreview,
        ).toHaveBeenCalledTimes(1),
      );
      await user.click(screen.getByTestId("model-target-codex"));
      await screen.findByRole("heading", { name: "Codex" });
      await act(async () => {
        if (result === "reject") reject(new Error("private late failure"));
        else
          resolve({
            ...applied,
            overall: result,
            providerState: result === "unknown" ? "unknown" : "applied",
            files: [
              {
                target: "claude_settings",
                state: result === "unknown" ? "unknown" : "applied",
              },
              { target: "claude_mcp", state: "conflict" },
            ],
          });
      });
      await user.click(screen.getByTestId("model-target-claude"));
      await screen.findByRole("heading", { name: "Claude Code" });
      expect(
        screen.getByRole("button", { name: "暂时无法确认当前设置" }),
      ).toBeDisabled();
      for (const label of ["配置名称", "服务地址", "API Key", "模型 ID"]) {
        expect(screen.getByLabelText(label)).toBeDisabled();
      }
      await user.type(screen.getByLabelText("API Key"), "blocked-fixture-key");
      expect(screen.getByLabelText("API Key")).toHaveValue("");
      await user.click(
        screen.getByRole("button", { name: "暂时无法确认当前设置" }),
      );
      expect(
        ports.providers.applyClaudeQuickSetupPreview,
      ).toHaveBeenCalledTimes(1);
      expect(ports.providers.previewClaudeQuickSetup).toHaveBeenCalledTimes(1);
      expect(ports.providers.applyQuickSetupWithResult).not.toHaveBeenCalled();
    },
  );

  it("settles an uncertain apply after the entire ModelsPage unmounts without restoring its UI", async () => {
    const { ports, user, view } = setup();
    let resolve!: (outcome: ClaudeQuickSetupOutcome) => void;
    ports.providers.applyClaudeQuickSetupPreview = vi.fn(
      () =>
        new Promise<ClaudeQuickSetupOutcome>((done) => {
          resolve = done;
        }),
    );
    await draft(user);
    await save(user);
    await confirm(user);
    await waitFor(() =>
      expect(
        ports.providers.applyClaudeQuickSetupPreview,
      ).toHaveBeenCalledTimes(1),
    );
    view.unmount();
    await act(async () => {
      resolve({
        ...applied,
        overall: "unknown",
        providerState: "unknown",
        files: [
          { target: "claude_settings", state: "unknown" },
          { target: "claude_mcp", state: "unknown" },
        ],
      });
    });
    expect(screen.queryByTestId("models-page")).toBeNull();
    expect(screen.queryByRole("dialog", { name: "保存前确认" })).toBeNull();
    expect(ports.providers.applyClaudeQuickSetupPreview).toHaveBeenCalledTimes(
      1,
    );
  });
});
