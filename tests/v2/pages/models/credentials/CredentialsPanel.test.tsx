import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { CredentialsPanel } from "@/v2/pages/models/credentials";
import { CANDIDATE_PLAN_BANNER } from "@/v2/shared/data/credentials";

function renderPanel(
  props: Parameters<typeof CredentialsPanel>[0] = {},
) {
  return render(<CredentialsPanel {...props} />);
}

function rowByName(name: string) {
  return screen.getByRole("option", { name: new RegExp(name) });
}

describe("CredentialsPanel public no-value surface", () => {
  it("never renders a secret textbox, password field, or paste copy", () => {
    renderPanel();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/password|密钥|api ?key/i)).not.toBeInTheDocument();
    expect(document.querySelector("input[type='password']")).toBeNull();
    expect(document.querySelector("textarea")).toBeNull();
    expect(screen.queryByText("粘贴密钥")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "凭据" })).toBeVisible();
    expect(screen.getByText("本机引用与无值状态 · 不显示密钥")).toBeVisible();
    expect(screen.getByRole("button", { name: "采集凭据" })).toBeVisible();
  });

  it("lists Provider, binding state, and next action for the four required rows", () => {
    renderPanel();
    const ready = rowByName("主编码");
    expect(ready).toHaveAttribute("data-binding-state", "bound");
    expect(ready).toHaveAttribute("data-availability", "ready");
    expect(within(ready).getByText("已绑定")).toBeVisible();
    expect(within(ready).getByText("可用")).toBeVisible();
    expect(within(ready).getByText("无")).toBeVisible();

    const legacy = rowByName("明文冲突");
    expect(legacy).toHaveAttribute("data-binding-state", "legacy");
    expect(within(legacy).getByText("明文待处理")).toBeVisible();
    expect(within(legacy).getByText("处理明文冲突")).toBeVisible();
    expect(screen.getByText("存在明文来源，先处理冲突")).toBeVisible();

    const unbound = rowByName("空引用");
    expect(unbound).toHaveAttribute("data-binding-state", "unbound");
    expect(within(unbound).getByText("未绑定")).toBeVisible();
    expect(within(unbound).getByText("采集凭据")).toBeVisible();

    const locked = rowByName("策略锁定");
    expect(locked).toHaveAttribute("data-binding-state", "bound");
    expect(locked).toHaveAttribute("data-availability", "locked");
    expect(within(locked).getByText("已锁定")).toBeVisible();
    expect(within(locked).getByText("解锁 FyAgent")).toBeVisible();
    expect(within(locked).queryByText("到系统解锁")).not.toBeInTheDocument();
  });

  it("paints staged candidate list rows as 等待变更计划, never 已绑定/可用", () => {
    renderPanel();
    for (const name of ["待计划", "待丢弃", "已过期"]) {
      const row = rowByName(name);
      expect(row).toHaveAttribute("data-staged-plan", "true");
      expect(within(row).getByText("等待变更计划")).toBeVisible();
      expect(within(row).queryByText("已绑定")).not.toBeInTheDocument();
      expect(within(row).queryByText("可用")).not.toBeInTheDocument();
      expect(within(row).queryByText("未绑定")).not.toBeInTheDocument();
    }
    const ready = rowByName("主编码");
    expect(ready).toHaveAttribute("data-staged-plan", "false");
    expect(within(ready).getByText("已绑定")).toBeVisible();
    expect(within(ready).getByText("可用")).toBeVisible();
  });

  it("shows the exact candidate banner and keeps pending discard as verifiedPendingPlan", async () => {
    const user = userEvent.setup();
    renderPanel();
    await user.click(rowByName("待计划"));
    const candidate = screen.getByTestId("credentials-candidate");
    expect(candidate).toHaveTextContent(CANDIDATE_PLAN_BANNER);
    expect(candidate.textContent?.indexOf(CANDIDATE_PLAN_BANNER)).toBe(0);
    expect(candidate).toHaveTextContent("核验同一凭据后迁移");
    expect(screen.getByRole("button", { name: "打开变更计划" })).toBeVisible();
    expect(screen.getByRole("button", { name: "丢弃候选" })).toBeVisible();

    await user.click(rowByName("待丢弃"));
    const pending = screen.getByTestId("credentials-candidate");
    expect(pending).toHaveAttribute("data-candidate-state", "verifiedPendingPlan");
    expect(pending).toHaveAttribute("data-pending-disposition", "discarded");
    expect(pending).toHaveTextContent(CANDIDATE_PLAN_BANNER);
    expect(pending).toHaveTextContent("后端条目仍可达");
    expect(within(pending).getByRole("button", { name: "丢弃候选" })).toBeVisible();
    expect(within(pending).queryByRole("button", { name: "打开变更计划" })).not.toBeInTheDocument();
  });

  it("uses different titles and confirm buttons for secret vs provider delete", async () => {
    const user = userEvent.setup();
    renderPanel({ initialOwnerId: "alpha-ready" });
    await user.click(screen.getByRole("button", { name: "删除本机凭据" }));
    const secretDialog = screen.getByRole("dialog");
    expect(within(secretDialog).getByRole("heading", { name: "删除本机凭据" })).toBeVisible();
    expect(within(secretDialog).getByRole("button", { name: "删除本机凭据" })).toBeVisible();
    expect(within(secretDialog).getByText("主编码")).toBeVisible();
    expect(within(secretDialog).getByText("共享只读")).toBeVisible();
    expect(within(secretDialog).getByText("无退路")).toBeVisible();
    expect(within(secretDialog).queryByRole("button", { name: "卸下 Provider" })).not.toBeInTheDocument();
    expect(document.activeElement).toHaveTextContent("取消");

    await user.click(within(secretDialog).getByRole("button", { name: "取消" }));
    await user.click(screen.getByRole("button", { name: "删除 Provider" }));
    const providerDialog = screen.getByRole("dialog");
    expect(within(providerDialog).getByRole("heading", { name: "删除 Provider" })).toBeVisible();
    expect(within(providerDialog).getByRole("button", { name: "卸下 Provider" })).toBeVisible();
    expect(within(providerDialog).getByText("只卸下该 Provider，凭据保留")).toBeVisible();
    expect(within(providerDialog).getByRole("button", { name: "单独删除凭据" })).toBeVisible();
    expect(within(providerDialog).queryByRole("heading", { name: "删除本机凭据" })).not.toBeInTheDocument();
  });

  it("exposes lockSource as a single unlock action", async () => {
    const user = userEvent.setup();
    renderPanel({ initialOwnerId: "delta-locked" });
    const policy = screen.getByTestId("credentials-status");
    expect(policy).toHaveAttribute("data-lock-source", "fyAgentPolicy");
    expect(within(policy).getByRole("button", { name: "解锁 FyAgent" })).toBeVisible();
    expect(within(policy).queryByRole("button", { name: "到系统解锁" })).not.toBeInTheDocument();
    expect(within(policy).queryByRole("button", { name: "解锁" })).not.toBeInTheDocument();

    await user.click(rowByName("系统锁定"));
    const backend = screen.getByTestId("credentials-status");
    expect(backend).toHaveAttribute("data-lock-source", "backend");
    expect(within(backend).getByRole("button", { name: "到系统解锁" })).toBeVisible();
    expect(within(backend).queryByRole("button", { name: "解锁 FyAgent" })).not.toBeInTheDocument();

    await user.click(rowByName("撤销项"));
    const revoked = screen.getByTestId("credentials-status");
    expect(revoked).toHaveAttribute("data-availability", "revoked");
    expect(revoked).toHaveTextContent("已撤销");
    expect(within(revoked).getByText(/用户删除/)).toBeVisible();
    expect(within(revoked).queryByText("缺失")).not.toBeInTheDocument();
    expect(within(revoked).queryByText("凭据缺失，重新采集")).not.toBeInTheDocument();
  });

  it("blocks provider delete confirmation when legacy sources remain", async () => {
    const user = userEvent.setup();
    renderPanel({ initialOwnerId: "beta-legacy" });
    await user.click(screen.getByRole("button", { name: "删除 Provider" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("data-impact-kind", "provider-blocked");
    expect(within(dialog).getByRole("heading", { name: "删除 Provider" })).toBeVisible();
    expect(within(dialog).queryByRole("button", { name: "卸下 Provider" })).not.toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: "删除本机凭据" })).not.toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "处理明文冲突" })).toBeVisible();
    expect(dialog.textContent).not.toMatch(/pdi_/);
  });

  it("shows the native capture waiting copy and returns focus to 采集凭据", async () => {
    const user = userEvent.setup();
    renderPanel();
    await user.click(screen.getByRole("button", { name: "采集凭据" }));
    expect(screen.getByTestId("credentials-capture-overlay")).toBeVisible();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "本机钥匙串" }));
    expect(screen.getByText("等待系统安全输入，应用内不会看到密钥")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.queryByTestId("credentials-capture-overlay")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "采集凭据" })).toHaveFocus();
  });

  it("keeps hardware confirmation in an overlay, never in the list or status card", () => {
    renderPanel({ initialOwnerId: "alpha-ready" });
    expect(screen.queryByTestId("credentials-hardware-overlay")).not.toBeInTheDocument();
    expect(screen.getByTestId("credentials-list")).not.toHaveTextContent("演示安全密钥");
    expect(screen.getByTestId("credentials-status")).not.toHaveTextContent("演示安全密钥");

    renderPanel({ initialOverlay: "hardware" });
    const overlay = screen.getByTestId("credentials-hardware-overlay");
    expect(overlay).toHaveTextContent("演示安全密钥");
    expect(overlay).toHaveTextContent("删除");
    expect(overlay).toHaveTextContent("30 秒");
    expect(within(overlay).getByRole("button", { name: "取消" })).toBeVisible();
  });

  it("renders empty copy without inventing a secret field", () => {
    renderPanel({ empty: true });
    expect(screen.getByText("还没有本机凭据引用")).toBeVisible();
    expect(screen.getAllByRole("button", { name: "采集凭据" }).length).toBeGreaterThan(0);
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });
});
