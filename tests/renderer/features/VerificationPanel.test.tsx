import {
  render,
  screen,
  waitFor,
  fireEvent,
  within,
} from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { FeatureProvider } from "@/shared/features/provider";
import { VerificationPanel } from "@/shared/features/verification/VerificationPanel";
import { PersistentSurface } from "@/shared/ui/PersistentSurface";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import {
  verificationFixture,
  evidenceFixture,
  PROJECT,
  OTHER_PROJECT,
} from "../fixtures/verification";

describe("project verification panel", () => {
  it("shows saved business metrics and specific sample failures", async () => {
    const ports = createBrowserFeaturePorts();
    const baseline = {
      ...evidenceFixture(),
      stage: "sample_passed" as const,
      checkerId: "kit_validator" as const,
      sourceClass: "local_fixture" as const,
      fixture: "baseline" as const,
      sample: {
        inputDigest: "a".repeat(64),
        code: "ok" as const,
        matchesExpectation: true,
        validator: "weekly-report/v1" as const,
        metrics: {
          currentMinor: 18000000,
          previousMinor: 15000000,
          growthBps: 2000,
          targetBps: 9000,
        },
        sourceRowIds: ["row-1" as const],
      },
    };
    ports.verification.get = vi.fn(async () => ({
      ...verificationFixture(),
      evidence: [
        baseline,
        {
          ...baseline,
          id: "00000000-0000-4000-8000-000000000004",
          outcome: "failed" as const,
          sample: {
            ...baseline.sample,
            code: "duplicate_row" as const,
            metrics: null,
            sourceRowIds: [],
          },
        },
      ],
    }));
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    await screen.findByText(/目标完成 90.00%/);
    expect(screen.getByText(/增长 20.00%/)).toBeVisible();
    expect(screen.getByText(/来源行：row-1/)).toBeVisible();
    expect(screen.getByText(/存在重复数据行/)).toBeVisible();
  });

  it("separates five unchecked stages and issues a saved-configuration check", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => verificationFixture());
    ports.verification.run = vi.fn(async () => ({
      ...verificationFixture(),
      evidence: [evidenceFixture()],
    }));
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    await waitFor(() =>
      expect(screen.getByText("检查保存的配置")).not.toBeDisabled(),
    );
    expect(screen.getAllByText("未检查")).toHaveLength(5);
    fireEvent.click(screen.getByText("检查保存的配置"));
    await screen.findByText("已保存，目标是否生效尚未确认");
    expect(
      within(screen.getByRole("region", { name: "客户已验收" })).getByText(
        "未检查",
      ),
    ).toBeVisible();
    expect(ports.verification.run).toHaveBeenCalledWith({
      projectId: PROJECT,
      expectedRevision: 1,
      checker: "saved_configuration_readback",
      runId: expect.any(String),
      fixture: null,
    });
  });
  it("does not fetch or dispatch while hidden, and isolates project switches", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async (id) => verificationFixture(id));
    const { rerender } = render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} active={false} />
      </FeatureProvider>,
    );
    expect(ports.verification.get).not.toHaveBeenCalled();
    expect(screen.getByText("检查保存的配置")).toBeDisabled();
    rerender(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={OTHER_PROJECT} />
      </FeatureProvider>,
    );
    await waitFor(() =>
      expect(ports.verification.get).toHaveBeenCalledWith(OTHER_PROJECT),
    );
    expect(ports.verification.get).not.toHaveBeenCalledWith(PROJECT);
  });
  it("allows scoped external customer acceptance and previews handoff", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => verificationFixture());
    ports.verification.record = vi.fn(async () => verificationFixture());
    ports.verification.preview = vi.fn(async () => ({
      snapshot: verificationFixture(),
      json: "{}",
      markdown: "# 交接\n待办：客户复核",
    }));
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    await waitFor(() =>
      expect(screen.getByText("登记检查或客户验收")).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByText("登记检查或客户验收"));
    for (const [label, value] of [
      ["登记或验收人", "张三"],
      ["角色", "客户负责人"],
      ["作用范围", "本机演练：经营周报样例，不是真实客户验收"],
      ["记录编号或文档链接", "https://example.feishu.cn/docx/Abc123"],
      ["出具记录的组织或人员", "客户业务部"],
    ])
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
    fireEvent.click(screen.getByText("保存人工记录"));
    await waitFor(() => expect(ports.verification.record).toHaveBeenCalled());
    expect(vi.mocked(ports.verification.record).mock.calls[0][0]).toMatchObject(
      {
        stage: "customer_accepted",
        scope: "本机演练：经营周报样例，不是真实客户验收",
        externalBasis: {
          reference: "https://example.feishu.cn/docx/Abc123",
          issuer: "客户业务部",
        },
        basisEvidenceIds: [],
      },
    );
    fireEvent.click(screen.getByText("预览脱敏交接"));
    await screen.findByText(/待办：客户复核/);
  });
  it("preserves the real manual form across a hidden project tab without dispatching", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => verificationFixture());
    ports.verification.record = vi.fn();
    const view = (active: boolean) => (
      <FeatureProvider ports={ports}>
        <PersistentSurface active={active}>
          <VerificationPanel projectId={PROJECT} />
        </PersistentSurface>
      </FeatureProvider>
    );
    const rendered = render(view(true));
    await waitFor(() =>
      expect(screen.getByText("登记检查或客户验收")).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByText("登记检查或客户验收"));
    const fields = [
      ["登记或验收人", "张三"],
      ["角色", "实施负责人"],
      ["作用范围", "本机演练：经营周报样例"],
      ["记录编号或文档链接", "UAT-9"],
      ["出具记录的组织或人员", "客户业务部"],
    ];
    for (const [label, value] of fields)
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
    const form = screen.getByRole("form", { name: "人工登记" });
    rendered.rerender(view(false));
    expect(form).toBeInTheDocument();
    expect(form).not.toBeVisible();
    expect(screen.getByLabelText("登记或验收人")).toBeDisabled();
    fireEvent.submit(form);
    expect(ports.verification.record).not.toHaveBeenCalled();
    rendered.rerender(view(true));
    await waitFor(() =>
      expect(screen.getByLabelText("登记或验收人")).not.toBeDisabled(),
    );
    expect(screen.getByRole("form", { name: "人工登记" })).toBe(form);
    for (const [label, value] of fields)
      expect(screen.getByLabelText(label)).toHaveValue(value);
    expect(ports.verification.record).not.toHaveBeenCalled();
  });
  it("keeps invalid manual text local and allows correction without refreshing", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => ({
      ...verificationFixture(),
      evidence: [evidenceFixture()],
    }));
    ports.verification.record = vi.fn(async () => verificationFixture());
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    await screen.findByText("通过");
    fireEvent.click(screen.getByText("登记检查或客户验收"));
    for (const [label, value] of [
      ["登记或验收人", "张三"],
      ["角色", "实施负责人"],
      ["作用范围", "/tmp/customer-report"],
      ["记录编号或文档链接", "UAT-9"],
      ["出具记录的组织或人员", "客户业务部"],
    ])
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
    fireEvent.click(screen.getByText("保存人工记录"));
    expect(await screen.findByRole("alert")).toHaveTextContent("作用范围");
    expect(ports.verification.record).not.toHaveBeenCalled();
    expect(screen.getByText("检查保存的配置")).not.toBeDisabled();
    expect(screen.getByText("保存人工记录")).not.toBeDisabled();
    expect(screen.getByLabelText("登记或验收人")).toHaveValue("张三");
    expect(screen.queryByText("待复核")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("作用范围"), {
      target: { value: "本机演练：经营周报；数据·样例。" },
    });
    fireEvent.change(screen.getByLabelText("记录编号或文档链接"), {
      target: { value: "https://example.com/report?token=private" },
    });
    fireEvent.click(screen.getByText("保存人工记录"));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "记录编号或文档链接",
    );
    expect(ports.verification.record).not.toHaveBeenCalled();
    fireEvent.change(screen.getByLabelText("记录编号或文档链接"), {
      target: { value: "https://example.feishu.cn/docx/Abc123" },
    });
    fireEvent.click(screen.getByText("保存人工记录"));
    await waitFor(() =>
      expect(ports.verification.record).toHaveBeenCalledTimes(1),
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(ports.verification.get).toHaveBeenCalledTimes(1);
  });
  it("validates handoff text locally and preserves the editable draft", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => verificationFixture());
    ports.verification.saveHandoff = vi.fn(async (request) => ({
      ...verificationFixture(),
      handoff: request.handoff,
      handoffRevision: 1,
    }));
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    fireEvent.click(await screen.findByText("添加待办"));
    fireEvent.change(screen.getByLabelText("事项 1"), {
      target: { value: "api_key：private" },
    });
    fireEvent.click(screen.getByText("保存交接事项"));
    expect(await screen.findByRole("alert")).toHaveTextContent("事项 1");
    expect(ports.verification.saveHandoff).not.toHaveBeenCalled();
    expect(screen.getByText("检查保存的配置")).not.toBeDisabled();
    fireEvent.change(screen.getByLabelText("事项 1"), {
      target: { value: "客户复核：周报口径；确认后交接。" },
    });
    fireEvent.click(screen.getByText("保存交接事项"));
    await waitFor(() =>
      expect(ports.verification.saveHandoff).toHaveBeenCalledTimes(1),
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByLabelText("事项 1")).toHaveValue(
      "客户复核：周报口径；确认后交接。",
    );
  });
  it("keeps unavailable adapters and failures explicit", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => ({
      ...verificationFixture(),
      available: false,
      projectRevision: null,
    }));
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    await screen.findByText(/当前项目暂不能检查/);
    expect(screen.getByText("检查保存的配置")).toBeDisabled();
  });
  it("keeps blocked projects readable and preserves manual and handoff drafts", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => ({
      ...verificationFixture(),
      evidence: [evidenceFixture()],
    }));
    ports.verification.run = vi.fn();
    ports.verification.record = vi.fn();
    ports.verification.saveHandoff = vi.fn();
    ports.verification.revoke = vi.fn();
    ports.verification.preview = vi.fn(async () => ({
      snapshot: verificationFixture(),
      json: "{}",
      markdown: "# 已保存的交接记录",
    }));
    ports.verification.export = vi.fn(async () => true);
    const view = (reason?: string) => (
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} mutationBlockedReason={reason} />
      </FeatureProvider>
    );
    const rendered = render(view());
    await screen.findByText("通过");
    fireEvent.click(screen.getByText("登记检查或客户验收"));
    fireEvent.change(screen.getByLabelText("登记或验收人"), {
      target: { value: "张三" },
    });
    fireEvent.click(screen.getByText("添加待办"));
    fireEvent.change(screen.getByLabelText("事项 1"), {
      target: { value: "客户复核：周报口径。" },
    });
    const reason = "请先保存项目资料，再进行检查或登记。";
    rendered.rerender(view(reason));
    expect(screen.getByText(reason)).toBeVisible();
    expect(
      within(screen.getByRole("region", { name: "配置已保存" })).getByText(
        "通过",
      ),
    ).toBeVisible();
    for (const action of [
      "检查保存的配置",
      "登记检查或客户验收",
      "保存人工记录",
      "保存交接事项",
      "撤销这条记录",
    ])
      expect(screen.getByText(action)).toBeDisabled();
    expect(screen.getByLabelText("登记或验收人")).toHaveValue("张三");
    expect(screen.getByLabelText("登记或验收人")).toBeDisabled();
    expect(screen.getByLabelText("事项 1")).toHaveValue("客户复核：周报口径。");
    fireEvent.click(screen.getByText("刷新"));
    await waitFor(() =>
      expect(ports.verification.get).toHaveBeenCalledTimes(2),
    );
    expect(ports.verification.run).not.toHaveBeenCalled();
    expect(ports.verification.record).not.toHaveBeenCalled();
    expect(ports.verification.saveHandoff).not.toHaveBeenCalled();
    expect(ports.verification.revoke).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.getByText("预览脱敏交接")).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByText("预览脱敏交接"));
    fireEvent.click(await screen.findByText("导出 Markdown"));
    await waitFor(() =>
      expect(ports.verification.export).toHaveBeenCalledWith(
        PROJECT,
        "markdown",
      ),
    );
    rendered.rerender(view());
    await waitFor(() =>
      expect(screen.getByLabelText("登记或验收人")).not.toBeDisabled(),
    );
    expect(screen.getByLabelText("登记或验收人")).toHaveValue("张三");
    expect(screen.getByLabelText("事项 1")).toHaveValue("客户复核：周报口径。");
  });
  it("marks cached passes for review after a failed mutation until explicit readback", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => ({
      ...verificationFixture(),
      evidence: [evidenceFixture()],
    }));
    ports.verification.run = vi.fn().mockRejectedValue("project_changed");
    render(
      <FeatureProvider ports={ports}>
        <VerificationPanel projectId={PROJECT} />
      </FeatureProvider>,
    );
    await screen.findByText("通过");
    fireEvent.click(screen.getByText("检查保存的配置"));
    await screen.findByText("待复核");
    expect(screen.queryByText("通过")).not.toBeInTheDocument();
    expect(screen.getByText("检查保存的配置")).toBeDisabled();
    fireEvent.click(screen.getByText("刷新"));
    await screen.findByText("通过");
    expect(screen.getByText("检查保存的配置")).not.toBeDisabled();
  });
});
