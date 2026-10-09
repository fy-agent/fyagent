import { useRef, useState } from "react";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { DatabaseRecoveryPort } from "@/shared/features/database-recovery";
import { DatabaseRecoveryDialog } from "@/shared/features/database-recovery-ui/DatabaseRecoveryDialog";
import { createDatabaseRecoveryPort } from "@/shared/platform/tauri/feature-ports/databaseRecovery";
import { Button } from "@/shared/ui/Button";
import { PersistentSurface } from "@/shared/ui/PersistentSurface";

const entry = { filename: "selected.db", sizeBytes: 4096, createdAt: "" };
const committed = {
  contractVersion: 1,
  phase: "readback",
  publication: "committed",
  safetyBackupFilename: "before-restore.db",
  warnings: [],
  resultCode: "restored",
};
const notCommitted = {
  ...committed,
  phase: "candidate",
  publication: "notCommitted",
  safetyBackupFilename: null,
  resultCode: "candidateFailed",
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function Harness({
  port,
  active = true,
}: {
  port: DatabaseRecoveryPort;
  active?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const originRef = useRef<HTMLButtonElement>(null);
  return (
    <PersistentSurface active={active}>
      <Button
        ref={originRef}
        dialogOriginRef={originRef}
        onClick={() => setOpen(true)}
      >
        打开本机恢复
      </Button>
      <DatabaseRecoveryDialog
        open={open}
        originRef={originRef}
        onClose={() => setOpen(false)}
        port={port}
      />
    </PersistentSurface>
  );
}

function setup() {
  const list = vi.fn(async (): Promise<unknown> => [entry]);
  const restore = vi
    .fn<(filename: unknown) => Promise<unknown>>()
    .mockResolvedValue(committed);
  const check = vi.fn(
    async (): Promise<unknown> => ({ contractVersion: 1, state: "readable" }),
  );
  const invoke = vi.fn(
    async (command: string, args?: Record<string, unknown>) => {
      if (command === "list_db_backups") return list();
      if (command === "restore_db_backup_outcome")
        return restore(args?.filename);
      if (command === "check_db_recovery_readability") return check();
      throw new Error("unexpected command");
    },
  );
  const port = createDatabaseRecoveryPort(invoke);
  const view = render(<Harness port={port} />);
  const user = userEvent.setup();
  const open = async () => {
    await user.click(screen.getByRole("button", { name: "打开本机恢复" }));
    await waitFor(() =>
      expect(
        screen.queryByRole("status", { name: "正在读取本机备份" }),
      ).not.toBeInTheDocument(),
    );
  };
  const select = async () => {
    await user.click(
      await screen.findByRole("radio", { name: /selected\.db/u }),
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "确认恢复" })).toBeEnabled(),
    );
  };
  return { user, port, view, invoke, list, restore, check, open, select };
}

describe("finite database recovery dialog through the native adapter", () => {
  it("discloses the actual selected leaf, cancels without writing and releases the modal", async () => {
    const { user, open, select, list, restore } = setup();
    expect(list).not.toHaveBeenCalled();
    await open();
    await select();
    expect(screen.getByText(/原副本会保留/u)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(document.body.style.pointerEvents).not.toBe("none");
    expect(restore).not.toHaveBeenCalled();
    await open();
    expect(screen.getByRole("button", { name: "确认恢复" })).toBeDisabled();
  });

  it("admits a double click once and sends only the filename", async () => {
    const { open, select, restore, invoke } = setup();
    const pending = deferred<unknown>();
    restore.mockImplementation(() => pending.promise);
    await open();
    await select();
    const confirm = screen.getByRole("button", { name: "确认恢复" });
    fireEvent.click(confirm);
    fireEvent.click(confirm);
    expect(restore).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenLastCalledWith("restore_db_backup_outcome", {
      filename: "selected.db",
    });
    await act(async () => pending.resolve(committed));
    expect(
      await screen.findByText(/数据库已恢复并完成读取检查/u),
    ).toBeVisible();
    expect(screen.getByText("before-restore.db")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "确认恢复" }),
    ).not.toBeInTheDocument();
  });

  it("allows one deliberate retry only for a proven notCommitted result", async () => {
    const { user, open, select, restore } = setup();
    restore.mockResolvedValue(notCommitted);
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    expect(await screen.findByText(/当前数据库未被替换/u)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "重试恢复" }));
    expect(await screen.findByText(/本次重试已用完/u)).toBeVisible();
    expect(restore).toHaveBeenCalledTimes(2);
    expect(
      screen.queryByRole("button", { name: "重试恢复" }),
    ).not.toBeInTheDocument();
  });

  it.each([
    {
      ...committed,
      resultCode: "readbackFailed",
      warnings: ["retentionFailed"],
    },
    {
      ...committed,
      phase: "publish",
      publication: "unknown",
      safetyBackupFilename: null,
      resultCode: "workerLost",
    },
    { ...committed, phase: "publish", resultCode: "workerLost" },
  ])(
    "keeps $publication blocked through list refresh and reopening",
    async (outcome) => {
      const { user, open, select, list, restore } = setup();
      restore.mockResolvedValue(outcome);
      await open();
      await select();
      await user.click(screen.getByRole("button", { name: "确认恢复" }));
      expect(await screen.findByText(/勿重复恢复/u)).toBeVisible();
      await user.click(screen.getByRole("button", { name: "刷新备份" }));
      await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
      expect(
        screen.queryByRole("button", { name: "重试恢复" }),
      ).not.toBeInTheDocument();
      await user.click(screen.getByRole("button", { name: "关闭" }));
      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
      await open();
      expect(
        screen.queryByRole("button", { name: "确认恢复" }),
      ).not.toBeInTheDocument();
      expect(restore).toHaveBeenCalledTimes(1);
      expect(
        screen.getByText(/列表可读不能确认数据库内容已恢复/u),
      ).toBeVisible();
    },
  );

  it("keeps a rejected IPC unknown and never displays the raw error", async () => {
    const { user, open, select, restore } = setup();
    restore.mockRejectedValue("SECRET-CANARY C:\\private\\database.db");
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    expect(await screen.findByText(/无法确认数据库是否已替换/u)).toBeVisible();
    expect(screen.queryByText(/CANARY/u)).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "重试恢复" }),
    ).not.toBeInTheDocument();
    expect(restore).toHaveBeenCalledTimes(1);
  });

  it("keeps unknown publication blocked even when the current database is readable", async () => {
    const { user, open, select, restore, check, invoke } = setup();
    restore.mockResolvedValue({
      ...committed,
      phase: "publish",
      publication: "unknown",
      safetyBackupFilename: null,
      resultCode: "workerLost",
    });
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    await user.click(
      await screen.findByRole("button", { name: "检查当前数据库" }),
    );
    expect(
      await screen.findByText(/当前数据库可读取。此检查不能确认/u),
    ).toBeVisible();
    expect(screen.getByText(/无法确认数据库是否已替换/u)).toBeVisible();
    expect(
      screen.queryByText(/数据库已恢复并完成读取检查/u),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "重试恢复" }),
    ).not.toBeInTheDocument();
    expect(check).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenLastCalledWith("check_db_recovery_readability");
    expect(restore).toHaveBeenCalledTimes(1);
  });

  it("reports an unavailable current database with finite manual read checks", async () => {
    const { user, open, select, restore, check } = setup();
    check.mockRejectedValueOnce("SECRET-CANARY");
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    await user.click(
      await screen.findByRole("button", { name: "检查当前数据库" }),
    );
    expect(await screen.findByText(/无法确认当前数据库可读取/u)).toBeVisible();
    expect(screen.queryByText(/CANARY/u)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "检查当前数据库" }));
    expect(
      await screen.findByText(/当前数据库可读取。此检查不能确认/u),
    ).toBeVisible();
    expect(check).toHaveBeenCalledTimes(2);
    expect(restore).toHaveBeenCalledTimes(1);
  });

  it("discards a late readability reply from a closed session", async () => {
    const { user, open, select, restore, check } = setup();
    const pending = deferred<unknown>();
    check.mockImplementationOnce(() => pending.promise);
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    await user.click(
      await screen.findByRole("button", { name: "检查当前数据库" }),
    );
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await act(async () =>
      pending.resolve({ contractVersion: 1, state: "readable" }),
    );
    await open();
    expect(
      screen.queryByText(/当前数据库可读取。此检查不能确认/u),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "确认恢复" }),
    ).not.toBeInTheDocument();
    expect(restore).toHaveBeenCalledTimes(1);
    expect(check).toHaveBeenCalledTimes(1);
  });

  it("preserves publication knowledge when closed before the native reply", async () => {
    const { user, open, select, restore } = setup();
    const pending = deferred<unknown>();
    restore.mockImplementation(() => pending.promise);
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await act(async () => pending.resolve(committed));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await open();
    expect(
      await screen.findByText(/数据库已恢复并完成读取检查/u),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "确认恢复" }),
    ).not.toBeInTheDocument();
    expect(restore).toHaveBeenCalledTimes(1);
  });

  it("hides the persistent portal and retains a late committed outcome without extra writes", async () => {
    const { user, port, view, open, select, restore, list } = setup();
    const pending = deferred<unknown>();
    restore.mockImplementation(() => pending.promise);
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    view.rerender(<Harness port={port} active={false} />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(list).toHaveBeenCalledTimes(1);
    await act(async () => pending.resolve(committed));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    view.rerender(<Harness port={port} />);
    expect(
      await screen.findByText(/数据库已恢复并完成读取检查/u),
    ).toBeVisible();
    expect(restore).toHaveBeenCalledTimes(1);
    expect(
      screen.queryByRole("button", { name: "确认恢复" }),
    ).not.toBeInTheDocument();
  });

  it("ignores an old list reply after close and reopening", async () => {
    const { user, open, list, restore } = setup();
    const pending = deferred<unknown>();
    list
      .mockImplementationOnce(() => pending.promise)
      .mockResolvedValue([{ ...entry, filename: "new-session.db" }]);
    await user.click(screen.getByRole("button", { name: "打开本机恢复" }));
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await open();
    expect(await screen.findByText("new-session.db")).toBeVisible();
    await act(async () => pending.resolve([entry]));
    expect(
      screen.queryByRole("radio", { name: /selected\.db/u }),
    ).not.toBeInTheDocument();
    expect(restore).not.toHaveBeenCalled();
  });

  it("refreshes empty and failed lists with read calls only", async () => {
    const { user, open, list, restore } = setup();
    list.mockRejectedValueOnce("SECRET-CANARY").mockResolvedValue([]);
    await open();
    expect(await screen.findByText(/无法读取本机备份列表/u)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "刷新备份" }));
    expect(await screen.findByText("没有可用的本机 .db 备份。")).toBeVisible();
    expect(list).toHaveBeenCalledTimes(2);
    expect(restore).not.toHaveBeenCalled();
  });

  it("blocks restore after host admission rejection until an explicit list refresh", async () => {
    const { user, open, select, invoke, list, restore } = setup();
    let initialized = false;
    list.mockImplementation(async () => {
      if (!initialized) throw new Error("AppState unavailable SECRET-CANARY");
      return [entry];
    });
    await open();
    expect(await screen.findByText(/无法读取本机备份列表/u)).toBeVisible();
    expect(screen.getByRole("button", { name: "确认恢复" })).toBeDisabled();
    expect(screen.queryByRole("radio")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    expect(invoke.mock.calls).toEqual([["list_db_backups"]]);
    expect(restore).not.toHaveBeenCalled();
    expect(
      screen.queryByText(/无法确认数据库是否已替换/u),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/SECRET-CANARY/u)).not.toBeInTheDocument();

    initialized = true;
    expect(list).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "确认恢复" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "刷新备份" }));
    await select();
    expect(invoke.mock.calls).toEqual([
      ["list_db_backups"],
      ["list_db_backups"],
    ]);
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    await waitFor(() =>
      expect(restore).toHaveBeenCalledExactlyOnceWith("selected.db"),
    );
    expect(invoke).toHaveBeenLastCalledWith("restore_db_backup_outcome", {
      filename: "selected.db",
    });
  });

  it("shows the four file types and current-device credential limitation", async () => {
    const { user, open, restore } = setup();
    await open();
    await user.click(screen.getByText("文件用途与限制"));
    for (const label of ["配置迁移包", "私有 .db 副本", "脱敏 SQL", "云快照"])
      expect(screen.getByText(label)).toBeVisible();
    expect(screen.getByText(/可能含登录凭据，请勿分享/u)).toBeVisible();
    expect(screen.getByText(/不能直接在其他设备复用/u)).toBeVisible();
    expect(restore).not.toHaveBeenCalled();
  });

  it("cancels through Escape with zero recovery calls", async () => {
    const { user, open, select, restore } = setup();
    await open();
    await select();
    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(document.body.style.pointerEvents).not.toBe("none");
    expect(restore).not.toHaveBeenCalled();
  });

  it("ignores a reply after the owning component is unmounted", async () => {
    const { user, view, open, select, restore } = setup();
    const pending = deferred<unknown>();
    restore.mockImplementation(() => pending.promise);
    await open();
    await select();
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    view.unmount();
    await act(async () => pending.resolve(committed));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(restore).toHaveBeenCalledTimes(1);
  });
});
