import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PersistentSurface } from "@/v2/shared/ui/PersistentSurface";
import { Dialog } from "@/v2/shared/ui/primitives";

describe("PersistentSurface", () => {
  it("hides portaled dialogs while inactive without treating hide as dismiss", () => {
    const onOpenChange = vi.fn();

    function Probe({ active }: { active: boolean }) {
      return (
        <PersistentSurface active={active}>
          <Dialog
            open
            title="待保存草稿"
            description="保活隐藏时不应关闭这份草稿。"
            onOpenChange={onOpenChange}
          >
            草稿内容
          </Dialog>
        </PersistentSurface>
      );
    }

    const view = render(<Probe active />);
    expect(screen.getByText("待保存草稿")).toBeVisible();

    view.rerender(<Probe active={false} />);
    expect(screen.queryByText("待保存草稿")).not.toBeInTheDocument();
    expect(onOpenChange).not.toHaveBeenCalled();

    view.rerender(<Probe active />);
    expect(screen.getByText("待保存草稿")).toBeVisible();
  });

  it("hides nested keep-alive dialogs when an ancestor surface is inactive", () => {
    render(
      <PersistentSurface active={false}>
        <PersistentSurface active>
          <Dialog
            open
            title="内层弹窗"
            description="祖先隐藏时内层弹窗也必须关掉。"
            onOpenChange={() => undefined}
          >
            内层内容
          </Dialog>
        </PersistentSurface>
      </PersistentSurface>,
    );

    expect(screen.queryByText("内层弹窗")).not.toBeInTheDocument();
  });
});
