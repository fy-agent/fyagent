import { useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Button, ConfirmDialog, Dialog } from "@/v2/shared/ui/primitives";
import { PersistentSurface } from "@/v2/shared/ui/PersistentSurface";

describe("shared desktop dialog", () => {
  it("uses the decision description without a filler body and restores focus", async () => {
    const user = userEvent.setup();
    function Example() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <Button onClick={() => setOpen(true)}>删除配置</Button>
          <ConfirmDialog
            open={open}
            title="删除此配置？"
            description="删除后需要重新添加。"
            onCancel={() => setOpen(false)}
            onConfirm={() => setOpen(false)}
          />
        </>
      );
    }
    render(<Example />);
    const trigger = screen.getByRole("button", { name: "删除配置" });
    await user.click(trigger);
    const dialog = screen.getByRole("dialog", { name: "删除此配置？" });
    expect(dialog).toHaveAccessibleDescription("删除后需要重新添加。");
    expect(dialog.querySelector(".fy-control-dialog-body")).toBeNull();
    expect(screen.queryByText("此操作需要你的明确确认。")).toBeNull();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "取消" })).toHaveFocus(),
    );
    await user.keyboard("{Escape}");
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("does not close or submit a pending confirmation", async () => {
    const onCancel = vi.fn();
    const onConfirm = vi.fn();
    render(
      <ConfirmDialog
        open
        pending
        title="删除配置？"
        description="正在处理此操作。"
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    );
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "处理中…" })).toBeDisabled();
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
    expect(onCancel).not.toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeVisible();
  });

  it("supports a title-only dialog and never presents a hidden route's portal", () => {
    const { rerender } = render(
      <PersistentSurface active>
        <Dialog open title="详细信息" onOpenChange={vi.fn()} size="comfortable">
          <p>内容</p>
        </Dialog>
      </PersistentSurface>,
    );
    expect(screen.getByRole("dialog")).not.toHaveAttribute("aria-describedby");
    expect(screen.getByRole("dialog")).toHaveClass(
      "fy-control-dialog-comfortable",
    );
    rerender(
      <PersistentSurface active={false}>
        <Dialog open title="详细信息" onOpenChange={vi.fn()}>
          <p>内容</p>
        </Dialog>
      </PersistentSurface>,
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
