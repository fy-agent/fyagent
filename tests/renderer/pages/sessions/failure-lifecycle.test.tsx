import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ImportPackageDialog } from "@/pages/sessions/components/ImportPackageDialog";
import { PersistentSurface } from "@/shared/ui/PersistentSurface";
import {
  sessionPackageSchema,
  type RestoreAttempt,
} from "@/shared/features/session-migration";
import { sessionPackageSample } from "../../../session-migration/samples";

it("keeps an unknown restore and its draft when a hidden page receives a late rejection", async () => {
  const user = userEvent.setup();
  let reject!: (error: unknown) => void;
  const pending = new Promise<RestoreAttempt[]>((_resolve, fail) => {
    reject = fail;
  });
  const onRestore = vi.fn(() => pending);
  const onReadPackage = vi.fn(async () => ({
    package: sessionPackageSchema.parse(sessionPackageSample()),
    attempts: [],
  }));
  const props = {
    open: true,
    onOpenChange: vi.fn(),
    onReadPackage,
    onRestore,
    onPickPackageFile: vi.fn(async () => "/tmp/draft-package.json"),
    onPickDirectory: vi.fn(async () => "/tmp/original-workspace"),
    localProbes: {
      codex: {
        providerId: "codex",
        installed: true,
        extractionSupported: true,
        writeSupported: true,
      },
    },
    initialTargetWorkspace: "/tmp/original-workspace",
    originRef: undefined,
  };
  function Harness({ active }: { active: boolean }) {
    return (
      <>
        <input aria-label="其它页面编辑处" />
        <PersistentSurface active={active}>
          <ImportPackageDialog {...props} />
        </PersistentSurface>
      </>
    );
  }
  const view = render(<Harness active />);
  await user.click(screen.getByRole("button", { name: "选择文件" }));
  await user.click(screen.getByRole("button", { name: "解析会话包" }));
  await screen.findByText("会话包解析成功", { exact: true });
  await user.click(screen.getByRole("button", { name: "确认恢复至目标软件" }));
  expect(onRestore).toHaveBeenCalledTimes(1);
  view.rerender(<Harness active={false} />);
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  const otherEditor = screen.getByRole("textbox", { name: "其它页面编辑处" });
  await user.click(otherEditor);
  await act(async () => {
    reject({ code: "nativeProtocolFailed", detail: { reason: "SECRET" } });
  });
  expect(otherEditor).toHaveFocus();
  expect(screen.queryByRole("dialog")).toBeNull();
  view.rerender(<Harness active />);
  await screen.findByText("目标恢复调用 · 未收到完整结果");
  expect(screen.getByLabelText("绑定目标机器工作区目录：")).toHaveValue(
    "/tmp/original-workspace",
  );
  expect(
    screen.getByRole("button", { name: "确认恢复至目标软件" }),
  ).toBeDisabled();
  expect(onRestore).toHaveBeenCalledTimes(1);
  expect(props.onOpenChange).not.toHaveBeenCalled();
  expect(screen.queryByText(/SECRET/u)).toBeNull();
});
