import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BulkAssignmentDialog } from "@/shared/features/controls/BulkAssignmentDialog";
import type {
  BulkAssignmentItem,
  BulkAssignmentPlan,
  BulkAssignmentResult,
} from "@/shared/features/bulk-assignment";

const items: BulkAssignmentItem[] = Array.from({ length: 70 }, (_, index) => ({
  id: `id-${index}`,
  name: index === 69 ? "unique-skill" : `skill-${index}`,
  identity: "original",
  apps: {},
}));

describe("bulk assignment selection and confirmation", () => {
  it("starts empty, preserves hidden selection across refresh, and confirms one of 70", async () => {
    const user = userEvent.setup();
    const execute = vi.fn<
      (
        plan: BulkAssignmentPlan,
        onResult: (result: BulkAssignmentResult) => void,
      ) => Promise<void>
    >(async () => {});
    const props = {
      kind: "Skills" as const,
      originRef: { current: null },
      busy: false,
      onClose: vi.fn(),
      onExecute: execute,
    };
    const { rerender } = render(
      <BulkAssignmentDialog {...props} items={items} />,
    );
    expect(screen.getByRole("button", { name: "预览所选 · 0" })).toBeDisabled();
    await user.type(
      screen.getByRole("searchbox", { name: "筛选批量资源" }),
      "unique",
    );
    await user.click(screen.getByRole("button", { name: "选择筛选结果" }));
    rerender(
      <BulkAssignmentDialog {...props} items={structuredClone(items)} />,
    );
    await user.clear(screen.getByRole("searchbox", { name: "筛选批量资源" }));
    await user.type(
      screen.getByRole("searchbox", { name: "筛选批量资源" }),
      "no-result",
    );
    expect(
      screen.getByText("没有筛选结果，已选项目仍保留。"),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "预览所选 · 1" }));
    expect(execute).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "确认执行 · 1" }));
    expect(execute).toHaveBeenCalledTimes(1);
    expect(
      execute.mock.calls[0][0].items.map((item: BulkAssignmentItem) => item.id),
    ).toEqual(["id-69"]);
  });

  it("cancel after preview performs zero writes", async () => {
    const user = userEvent.setup();
    const execute = vi.fn<
      (
        plan: BulkAssignmentPlan,
        onResult: (result: BulkAssignmentResult) => void,
      ) => Promise<void>
    >(async () => {});
    const close = vi.fn();
    render(
      <BulkAssignmentDialog
        kind="MCP"
        originRef={{ current: null }}
        busy={false}
        items={items.slice(0, 1)}
        onClose={close}
        onExecute={execute}
      />,
    );
    await user.click(screen.getByRole("button", { name: "选择筛选结果" }));
    await user.click(screen.getByRole("button", { name: "预览所选 · 1" }));
    await user.click(screen.getByRole("button", { name: "关闭" }));
    expect(close).toHaveBeenCalledOnce();
    expect(execute).not.toHaveBeenCalled();
  });
  it("does not offer the same confirmation again when execution returns no row results", async () => {
    const user = userEvent.setup();
    const execute = vi.fn<
      (
        plan: BulkAssignmentPlan,
        onResult: (result: BulkAssignmentResult) => void,
      ) => Promise<void>
    >(async () => {});
    render(
      <BulkAssignmentDialog
        kind="Skills"
        originRef={{ current: null }}
        busy={false}
        items={items.slice(0, 1)}
        onClose={vi.fn()}
        onExecute={execute}
      />,
    );
    await user.click(screen.getByRole("button", { name: "选择筛选结果" }));
    await user.click(screen.getByRole("button", { name: "预览所选 · 1" }));
    await user.click(screen.getByRole("button", { name: "确认执行 · 1" }));
    expect(
      await screen.findByText(
        "读取或执行失败；不能确认完成。请刷新后重新预览。",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "确认执行 · 1" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "选择未完成项重新预览" }),
    ).toBeEnabled();
  });
});
