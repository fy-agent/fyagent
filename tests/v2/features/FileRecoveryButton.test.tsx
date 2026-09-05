import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { FileRecoveryButton } from "@/v2/shared/features/controls/FileRecoveryButton";
import type { ConfigRecoverySnapshot } from "@/v2/shared/features/config-recovery";
import { FeatureProvider } from "@/v2/shared/features/provider";
import { createBrowserFeaturePorts } from "@/v2/shared/platform/browser/features";
import { TooltipProvider } from "@/v2/shared/ui/primitives";

const receiptId = "323e4567-e89b-42d3-a456-426614174000";
function setup(overrides: Partial<ConfigRecoverySnapshot> = {}) {
  const ports = createBrowserFeaturePorts();
  const snapshot: ConfigRecoverySnapshot = {
    contractVersion: 1,
    target: "codex_auth",
    writeTarget: {
      path: "~/.codex/auth.json",
      backupPath: "~/.codex/auth.json.fyagent.backup",
      exists: true,
    },
    state: "available",
    receiptId,
    restoresExistingFile: true,
    ...overrides,
  };
  const list = vi.fn(async () => [snapshot]);
  const restore = vi.fn(async () => ({
    ...snapshot,
    state: "none" as const,
    receiptId: null,
    restoresExistingFile: null,
  }));
  ports.configRecovery = { list, restore };
  const onRestored = vi.fn(async () => undefined);
  render(
    <TooltipProvider>
      <FeatureProvider ports={ports}>
        <FileRecoveryButton targets={["codex_auth"]} onRestored={onRestored} />
      </FeatureProvider>
    </TooltipProvider>,
  );
  return { list, restore, onRestored };
}

describe("file recovery confirmation", () => {
  it("discloses files before confirmation, cancels without writing and restores once", async () => {
    const user = userEvent.setup();
    const { list, restore, onRestored } = setup();
    expect(list).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "撤回文件修改" }));
    expect(
      await screen.findByText("~/.codex/auth.json.fyagent.backup"),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "确认恢复文件" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(restore).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "撤回文件修改" }));
    await user.click(
      await screen.findByRole("radio", { name: "Codex 登录凭证" }),
    );
    const confirm = screen.getByRole("button", { name: "确认恢复文件" });
    await waitFor(() => expect(confirm).toBeEnabled());
    await user.dblClick(confirm);
    await waitFor(() => expect(onRestored).toHaveBeenCalledTimes(1));
    expect(restore).toHaveBeenCalledTimes(1);
    expect(restore).toHaveBeenCalledWith({ target: "codex_auth", receiptId });
    expect(await screen.findByText(/文件已恢复到上次修改前/)).toBeVisible();
  });

  it("never offers to overwrite externally changed files", async () => {
    const user = userEvent.setup();
    const { restore } = setup({
      state: "conflict",
      receiptId: null,
      restoresExistingFile: null,
    });
    await user.click(screen.getByRole("button", { name: "撤回文件修改" }));
    expect(
      await screen.findByRole("radio", { name: "Codex 登录凭证" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "确认恢复文件" })).toBeDisabled();
    expect(screen.getByText(/文件或备份已有变化/)).toBeVisible();
    expect(restore).not.toHaveBeenCalled();
  });

  it("explains that undoing first creation deletes the new file", async () => {
    const user = userEvent.setup();
    const { restore } = setup({ restoresExistingFile: false });
    await user.click(screen.getByRole("button", { name: "撤回文件修改" }));
    await user.click(
      await screen.findByRole("radio", { name: "Codex 登录凭证" }),
    );
    expect(screen.getByText(/此前没有此文件/)).toBeVisible();
    expect(
      screen.getByRole("button", { name: "确认删除新文件" }),
    ).toBeEnabled();
    expect(restore).not.toHaveBeenCalled();
  });
});
