import { useState } from "react";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Button, ConfirmDialog, Dialog } from "@/v2/shared/ui/primitives";
import { PersistentSurface } from "@/v2/shared/ui/PersistentSurface";
import { TabsPrimitive } from "@/v2/shared/ui/vendor";

describe("shared desktop dialog", () => {
  it("returns a rejected automatic tab change to the selected tab without reopening confirmation", async () => {
    const user = userEvent.setup();
    const frames: FrameRequestCallback[] = [];
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      frames.push(callback);
      return frames.length;
    });
    function Example() {
      const [value, setValue] = useState("one");
      const [candidate, setCandidate] = useState<string | null>(null);
      return (
        <>
          <TabsPrimitive.Root
            value={value}
            onValueChange={(next) => {
              if (next !== value) setCandidate(next);
            }}
          >
            <TabsPrimitive.List aria-label="编辑视图">
              <TabsPrimitive.Trigger value="one">
                当前编辑
              </TabsPrimitive.Trigger>
              <TabsPrimitive.Trigger value="two">
                其他编辑
              </TabsPrimitive.Trigger>
            </TabsPrimitive.List>
          </TabsPrimitive.Root>
          <ConfirmDialog
            open={candidate !== null}
            title="放弃更改？"
            description="编辑尚未保存。"
            onCancel={() => setCandidate(null)}
            onConfirm={() => {
              if (candidate) setValue(candidate);
              setCandidate(null);
            }}
          />
        </>
      );
    }
    render(<Example />);
    await user.tab();
    expect(screen.getByRole("tab", { name: "当前编辑" })).toHaveFocus();
    await user.keyboard("{ArrowRight}");
    await user.click(await screen.findByRole("button", { name: "取消" }));
    await waitFor(() => expect(frames.length).toBeGreaterThan(0));
    await act(async () => {
      frames.splice(0).forEach((callback) => callback(0));
    });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("tab", { name: "当前编辑" })).toHaveFocus();
    expect(screen.getByRole("tab", { name: "当前编辑" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("does not let an old close frame steal focus from a newly opened dialog", async () => {
    const user = userEvent.setup();
    const frames: FrameRequestCallback[] = [];
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      frames.push(callback);
      return frames.length;
    });
    function Example() {
      const [first, setFirst] = useState(false);
      const [second, setSecond] = useState(false);
      return (
        <>
          <Button onClick={() => setFirst(true)}>打开第一个</Button>
          <Dialog open={first} onOpenChange={setFirst} title="第一个">
            <Button
              onClick={() => {
                setFirst(false);
                setSecond(true);
              }}
            >
              转到第二个
            </Button>
          </Dialog>
          <Dialog open={second} onOpenChange={setSecond} title="第二个">
            <input aria-label="第二个输入" />
          </Dialog>
        </>
      );
    }
    render(<Example />);
    const trigger = screen.getByRole("button", { name: "打开第一个" });
    await user.click(trigger);
    await user.click(screen.getByRole("button", { name: "转到第二个" }));
    await screen.findByRole("dialog", { name: "第二个" });
    await waitFor(() => expect(frames.length).toBeGreaterThan(0));
    const focus = vi.spyOn(trigger, "focus");
    await act(async () => {
      frames.splice(0).forEach((callback) => callback(0));
    });
    expect(focus).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox", { name: "第二个输入" })).toHaveFocus();
  });
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
