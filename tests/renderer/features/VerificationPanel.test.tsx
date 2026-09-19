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
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import {
  verificationFixture,
  evidenceFixture,
  PROJECT,
  OTHER_PROJECT,
} from "../fixtures/verification";

describe("project verification panel", () => {
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
      ["作用范围", "周报试点"],
      ["外部真实记录编号", "UAT-9"],
      ["出具记录的组织或人员", "客户业务部"],
    ])
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
    fireEvent.click(screen.getByText("保存人工记录"));
    await waitFor(() => expect(ports.verification.record).toHaveBeenCalled());
    expect(vi.mocked(ports.verification.record).mock.calls[0][0]).toMatchObject(
      {
        stage: "customer_accepted",
        externalBasis: { reference: "UAT-9", issuer: "客户业务部" },
        basisEvidenceIds: [],
      },
    );
    fireEvent.click(screen.getByText("预览脱敏交接"));
    await screen.findByText(/待办：客户复核/);
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
  it("marks cached passes for review after a failed mutation until explicit readback", async () => {
    const ports = createBrowserFeaturePorts();
    ports.verification.get = vi.fn(async () => ({ ...verificationFixture(), evidence: [evidenceFixture()] }));
    ports.verification.run = vi.fn().mockRejectedValue("project_changed");
    render(<FeatureProvider ports={ports}><VerificationPanel projectId={PROJECT} /></FeatureProvider>);
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
