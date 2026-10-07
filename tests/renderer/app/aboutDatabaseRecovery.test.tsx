import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  DatabaseRecoveryPort,
  DatabaseRestoreOutcome,
} from "@/shared/features/database-recovery";
import { FeatureProvider } from "@/shared/features/provider";
import {
  createBrowserFeaturePorts,
  NATIVE_ONLY_ERROR,
} from "@/shared/platform/browser/features";
import { TopBar } from "@/widgets/app-shell/TopBar";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const filename = "backup-20261007.db";
const backup = { filename, sizeBytes: 1024, createdAt: "2026-10-07T00:00:00Z" };
const committed: DatabaseRestoreOutcome = {
  contractVersion: 1,
  phase: "readback",
  publication: "committed",
  safetyBackupFilename: "before-restore-20261007.db",
  warnings: [],
  resultCode: "restored",
};
const notCommitted: DatabaseRestoreOutcome = {
  contractVersion: 1,
  phase: "candidate",
  publication: "notCommitted",
  safetyBackupFilename: null,
  warnings: [],
  resultCode: "candidateFailed",
};

function fixture(
  restore = vi.fn<DatabaseRecoveryPort["restore"]>(async () => committed),
) {
  const ports = createBrowserFeaturePorts();
  const list = vi.fn(async () => [backup]);
  const checkReadability = vi.fn<DatabaseRecoveryPort["checkReadability"]>(
    async () => ({ contractVersion: 1, state: "readable" }),
  );
  ports.settings.getAppVersion = vi.fn(async () => "9.8.7");
  ports.databaseRecovery = { list, restore, checkReadability };
  render(
    <FeatureProvider ports={ports}>
      <TopBar />
    </FeatureProvider>,
  );
  const user = userEvent.setup();
  const trigger = screen.getByRole("button", { name: "关于 FyAgent" });
  // jsdom has no layout. Give only the actual source and persistent return
  // controls finite geometry, so the existing transient-origin contract runs.
  vi.spyOn(trigger, "getBoundingClientRect").mockReturnValue(
    new DOMRect(600, 20, 80, 32),
  );
  const open = async () => {
    await user.click(trigger);
    await act(async () => {
      await vi.dynamicImportSettled();
    });
    const about = await screen.findByRole("dialog", { name: "关于 FyAgent" });
    const recoveryTrigger = within(about).getByRole("button", {
      name: "本机备份与恢复",
    });
    vi.spyOn(recoveryTrigger, "getBoundingClientRect").mockReturnValue(
      new DOMRect(240, 200, 160, 32),
    );
    await user.click(recoveryTrigger);
    await act(async () => {
      await vi.dynamicImportSettled();
    });
    const dialog = await screen.findByRole("dialog", {
      name: "本机备份与恢复",
    });
    await waitFor(() =>
      expect(
        within(dialog).queryByText("正在读取本机备份") === null ||
          within(dialog).queryByText(/关闭对话不会取消已经开始的恢复/u) !==
            null,
      ).toBe(true),
    );
    return dialog;
  };
  const close = async (dialog: HTMLElement) => {
    await user.click(within(dialog).getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(trigger).toHaveFocus());
  };
  return { user, open, close, list, restore, checkReadability };
}

beforeEach(() => invoke.mockReset());

describe("About database recovery composition", () => {
  it("opens the public entry without a write, requires a selected backup, and returns focus on Escape", async () => {
    const { user, open, list, restore } = fixture();
    expect(list).not.toHaveBeenCalled();
    const dialog = await open();
    expect(
      screen.queryByRole("dialog", { name: "关于 FyAgent" }),
    ).not.toBeInTheDocument();
    await waitFor(() =>
      expect(
        within(dialog).getByRole("button", { name: "关闭" }),
      ).toHaveFocus(),
    );
    expect(list).toHaveBeenCalledTimes(1);
    expect(
      within(dialog).getByRole("button", { name: "确认恢复" }),
    ).toBeDisabled();
    await user.click(within(dialog).getByRole("radio"));
    expect(
      within(dialog).getByRole("button", { name: "确认恢复" }),
    ).toBeEnabled();
    expect(restore).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "关于 FyAgent" }),
      ).toHaveFocus(),
    );
    expect(restore).not.toHaveBeenCalled();
  });

  it.each(["committed", "unknown"] as const)(
    "retains %s across close/reopen and never offers another restore after a readable check",
    async (publication) => {
      const restore = vi.fn<DatabaseRecoveryPort["restore"]>(async () => {
        if (publication === "unknown")
          throw new Error("private transport error");
        return committed;
      });
      const { user, open, close, checkReadability } = fixture(restore);
      let dialog = await open();
      await user.click(within(dialog).getByRole("radio"));
      await user.click(
        within(dialog).getByRole("button", { name: "确认恢复" }),
      );
      const result =
        publication === "committed"
          ? "数据库已恢复并完成读取检查"
          : "无法确认数据库是否已替换";
      expect(await within(dialog).findByText(new RegExp(result))).toBeVisible();
      await close(dialog);
      dialog = await open();
      expect(within(dialog).getByText(new RegExp(result))).toBeVisible();
      expect(
        within(dialog).queryByRole("button", { name: /确认恢复|重试恢复/u }),
      ).not.toBeInTheDocument();
      await user.click(
        within(dialog).getByRole("button", { name: "检查当前数据库" }),
      );
      expect(
        await within(dialog).findByText(/当前数据库可读取/u),
      ).toBeVisible();
      expect(checkReadability).toHaveBeenCalledTimes(1);
      expect(within(dialog).getByText(new RegExp(result))).toBeVisible();
      expect(
        within(dialog).queryByRole("button", { name: /确认恢复|重试恢复/u }),
      ).not.toBeInTheDocument();
      expect(restore).toHaveBeenCalledExactlyOnceWith(filename);
      expect(
        screen.queryByText("private transport error"),
      ).not.toBeInTheDocument();
    },
  );

  it("keeps an in-flight restore through close/reopen without admitting a second write", async () => {
    let finish!: (outcome: DatabaseRestoreOutcome) => void;
    const pending = new Promise<DatabaseRestoreOutcome>((resolve) => {
      finish = resolve;
    });
    const restore = vi.fn<DatabaseRecoveryPort["restore"]>(() => pending);
    const { user, open, close } = fixture(restore);
    let dialog = await open();
    await user.click(within(dialog).getByRole("radio"));
    await user.click(within(dialog).getByRole("button", { name: "确认恢复" }));
    expect(
      await within(dialog).findByText(/关闭对话不会取消已经开始的恢复/u),
    ).toBeVisible();
    await close(dialog);
    dialog = await open();
    expect(
      within(dialog).getByText(/关闭对话不会取消已经开始的恢复/u),
    ).toBeVisible();
    expect(
      within(dialog).getByRole("button", { name: "刷新备份" }),
    ).toBeDisabled();
    expect(
      within(dialog).queryByRole("button", { name: /确认恢复|重试恢复/u }),
    ).not.toBeInTheDocument();
    await act(async () => {
      finish(committed);
      await pending;
    });
    expect(
      await within(dialog).findByText(/数据库已恢复并完成读取检查/u),
    ).toBeVisible();
    expect(restore).toHaveBeenCalledExactlyOnceWith(filename);
  });

  it("retains the finite retry budget across reopening after proven uncommitted failures", async () => {
    const restore = vi.fn<DatabaseRecoveryPort["restore"]>(
      async () => notCommitted,
    );
    const { user, open, close } = fixture(restore);
    let dialog = await open();
    for (let attempt = 0; attempt < 2; attempt += 1) {
      await user.click(within(dialog).getByRole("radio"));
      await user.click(
        within(dialog).getByRole("button", {
          name: attempt === 0 ? "确认恢复" : "重试恢复",
        }),
      );
      expect(
        await within(dialog).findByText(/备份未能完成恢复准备/u),
      ).toBeVisible();
      await close(dialog);
      dialog = await open();
    }
    expect(within(dialog).getByText(/本次重试已用完/u)).toBeVisible();
    expect(
      within(dialog).queryByRole("button", { name: /确认恢复|重试恢复/u }),
    ).not.toBeInTheDocument();
    expect(restore).toHaveBeenCalledTimes(2);
    expect(restore).toHaveBeenNthCalledWith(1, filename);
    expect(restore).toHaveBeenNthCalledWith(2, filename);
  });

  it("composes the native recovery port with filename-only command payloads and independent read-only checks", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/shared/platform/tauri/features"
    );
    invoke
      .mockResolvedValueOnce([backup])
      .mockResolvedValueOnce(committed)
      .mockResolvedValueOnce({ contractVersion: 1, state: "readable" });
    const ports = createTauriFeaturePorts();
    expect(await ports.databaseRecovery.list()).toEqual([backup]);
    expect(await ports.databaseRecovery.restore(filename)).toEqual(committed);
    expect(await ports.databaseRecovery.checkReadability()).toEqual({
      contractVersion: 1,
      state: "readable",
    });
    expect(
      invoke.mock.calls.map(([command, args]) => ({ command, args })),
    ).toEqual([
      { command: "list_db_backups", args: undefined },
      { command: "restore_db_backup_outcome", args: { filename } },
      { command: "check_db_recovery_readability", args: undefined },
    ]);
    await expect(
      ports.databaseRecovery.restore("../private.db"),
    ).rejects.toThrow();
    expect(invoke).toHaveBeenCalledTimes(3);
  });

  it("rejects every browser recovery operation as native-only", async () => {
    const port = createBrowserFeaturePorts().databaseRecovery;
    await expect(port.list()).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(port.restore(filename)).rejects.toThrow(NATIVE_ONLY_ERROR);
    await expect(port.checkReadability()).rejects.toThrow(NATIVE_ONLY_ERROR);
    expect(invoke).not.toHaveBeenCalled();
  });
});
