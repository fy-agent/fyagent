import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ApplyWorkspace } from "@/v2/pages/models/apply/ApplyWorkspace";
import {
  APPLY_SCENARIOS,
  applyFixtures,
  type ApplyScenario,
} from "@/v2/pages/models/apply/fixtures";

const STEP_IDS = [
  "verify_plan",
  "backup_resources",
  "write_managed_config",
  "readback_verify",
  "refresh_local_state",
] as const;

const STEP_LABELS = [
  "核对计划",
  "备份资源",
  "写入受管配置",
  "回读核对",
  "刷新本机状态",
] as const;

const SUCCESS_TITLE = "配置已应用，可直接开始使用";
const NO_USAGE_EVIDENCE = "配置已应用，尚无真实使用证据";
const FORBIDDEN_FIELD = /sk-|\/Users\/|[A-Za-z]:\\|SecretRef|SecretBackend|secretValue|secretHash|planDigest|absolutePath|promptBody|memoryBody|configText|rawDiff|providerRequestId/;

function renderApply(scenario: ApplyScenario) {
  render(<ApplyWorkspace scenario={scenario} />);
}

function serializedFixtures(): string {
  return JSON.stringify(applyFixtures);
}

describe("ApplyWorkspace presentation fixtures", () => {
  it("exports running, succeeded, warning, failed, and cancelled snapshots", () => {
    expect(APPLY_SCENARIOS).toEqual([
      "running",
      "succeeded",
      "warning",
      "failed",
      "cancelled",
    ]);
    expect(Object.keys(applyFixtures).sort()).toEqual([...APPLY_SCENARIOS].sort());
  });

  it("keeps five fixed steps, at most one running step, and no forbidden fields", () => {
    expect(serializedFixtures()).not.toMatch(FORBIDDEN_FIELD);

    for (const scenario of APPLY_SCENARIOS) {
      const snapshot = applyFixtures[scenario];
      expect(snapshot.steps.map((step) => step.stepId)).toEqual([...STEP_IDS]);
      expect(
        snapshot.steps.filter((step) => step.status === "running"),
      ).toHaveLength(scenario === "running" ? 1 : 0);
    }
  });
});

describe("ApplyWorkspace status, effect, copy, and actions", () => {
  it("renders the three-pane prototype workspace without a new top-level nav", () => {
    renderApply("succeeded");

    expect(screen.getByRole("heading", { name: "应用配置" })).toBeVisible();
    expect(screen.getByText("前端原型 · 模拟数据")).toBeVisible();
    expect(screen.getByText("不发送测试请求")).toBeVisible();
    expect(screen.getByTestId("apply-workspace")).toHaveAttribute(
      "data-data-source",
      "prototype",
    );
    expect(screen.getByTestId("apply-plan")).toBeVisible();
    expect(screen.getByTestId("apply-timeline")).toBeVisible();
    expect(screen.getByTestId("apply-outcome")).toBeVisible();
    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: "应用配置" }),
    ).not.toBeInTheDocument();
  });

  it("shows the frozen success title and no-usage evidence on succeeded", () => {
    renderApply("succeeded");

    const workspace = screen.getByTestId("apply-workspace");
    expect(workspace).toHaveAttribute("data-status", "succeeded");
    expect(workspace).toHaveAttribute("data-effect", "applied");
    expect(
      screen.getByRole("heading", { name: SUCCESS_TITLE }),
    ).toBeVisible();
    expect(screen.getByText("已完成本机写入与回读核对")).toBeVisible();
    expect(screen.getByText(NO_USAGE_EVIDENCE)).toBeVisible();
    expect(screen.getByText("效果：已应用")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "完成并开始使用" }),
    ).toBeVisible();
  });

  it("keeps the success title on warning and offers retry refresh", () => {
    renderApply("warning");

    const workspace = screen.getByTestId("apply-workspace");
    expect(workspace).toHaveAttribute("data-status", "warning");
    expect(workspace).toHaveAttribute("data-effect", "applied");
    expect(
      screen.getByRole("heading", { name: SUCCESS_TITLE }),
    ).toBeVisible();
    expect(
      screen.getByText("核心配置已回读一致；仍有一项本机辅助状态待处理"),
    ).toBeVisible();
    expect(screen.getByText(NO_USAGE_EVIDENCE)).toBeVisible();
    expect(
      screen.getByRole("button", { name: "完成并开始使用" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "重试辅助刷新" }),
    ).toBeVisible();
  });

  it("shows failed recovery without a green success title", () => {
    renderApply("failed");

    const workspace = screen.getByTestId("apply-workspace");
    expect(workspace).toHaveAttribute("data-status", "failed");
    expect(workspace).toHaveAttribute("data-effect", "unknown");
    expect(
      screen.getByRole("heading", { name: "无法确认配置结果" }),
    ).toBeVisible();
    expect(
      screen.getByText("不会自动重复写入；请先重试回读或恢复备份"),
    ).toBeVisible();
    expect(screen.getByText("效果：结果未确认")).toBeVisible();
    expect(screen.getByText("备份可用")).toBeVisible();
    expect(screen.getByRole("button", { name: "重试回读" })).toBeVisible();
    expect(screen.getByRole("button", { name: "恢复备份" })).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: SUCCESS_TITLE }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(NO_USAGE_EVIDENCE)).not.toBeInTheDocument();
  });

  it("shows cancelled with no write effect and a return-to-plan action", () => {
    renderApply("cancelled");

    const workspace = screen.getByTestId("apply-workspace");
    expect(workspace).toHaveAttribute("data-status", "cancelled");
    expect(workspace).toHaveAttribute("data-effect", "none");
    expect(screen.getByRole("heading", { name: "已取消应用" })).toBeVisible();
    expect(screen.getByText("尚未开始受管配置写入")).toBeVisible();
    expect(screen.getByText("效果：无变更")).toBeVisible();
    expect(screen.getByRole("button", { name: "返回计划" })).toBeVisible();
    expect(
      screen.queryByRole("heading", { name: SUCCESS_TITLE }),
    ).not.toBeInTheDocument();
  });

  it("renders running before write with one current step and a cancel action", () => {
    renderApply("running");

    const workspace = screen.getByTestId("apply-workspace");
    expect(workspace).toHaveAttribute("data-status", "running");
    expect(workspace).toHaveAttribute("data-effect", "none");
    expect(screen.getByRole("heading", { name: "正在应用配置" })).toBeVisible();
    expect(screen.getByText("正在核对计划并保护可恢复性")).toBeVisible();
    expect(screen.getByRole("button", { name: "请求取消" })).toBeVisible();

    const timeline = screen.getByTestId("apply-timeline");
    expect(within(timeline).getAllByRole("listitem")).toHaveLength(5);
    expect(within(timeline).getByRole("listitem", { current: "step" })).toHaveTextContent(
      "备份资源",
    );
  });

  it("labels every step in Chinese so status is not color-only", () => {
    renderApply("succeeded");

    const timeline = screen.getByTestId("apply-timeline");
    for (const label of STEP_LABELS) {
      expect(within(timeline).getByText(label)).toBeVisible();
    }
    expect(within(timeline).getAllByText("已完成")).toHaveLength(5);
  });
});
