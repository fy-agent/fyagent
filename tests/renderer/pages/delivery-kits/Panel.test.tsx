import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import fixture from "../../../fixtures/deliveryKitContract.v1.json";
import {
  ProjectDeliveryKitsPanel,
  type ProjectDeliveryKitsPanelProps,
} from "@/shared/features/delivery-kits-ui/ProjectDeliveryKitsPanel";
import {
  parseKitView,
  type KitDemoResult,
  type KitPreview,
} from "@/domain/delivery-kits";
import type { DeliveryKitsPort } from "@/shared/features/delivery-kits";

const kit = parseKitView(fixture);
const preview: KitPreview = {
  previewId: "11111111-1111-4111-8111-111111111111",
  kind: "import",
  kit,
  conflict: false,
};
const demo: KitDemoResult = {
  ...kit.identity,
  sourceClass: "local_fixture",
  validatorVersion: "weekly-report/v1",
  checkedAt: "2026-09-19T12:00:00+00:00",
  cases: [
    {
      fixtureId: "baseline",
      inputDigest: "a".repeat(64),
      code: "ok",
      metrics: {
        currentMinor: 18000000,
        previousMinor: 15000000,
        growthBps: 2000,
        targetBps: 9000,
      },
      sourceRowIds: ["row-1", "row-2"],
      matchesExpectation: true,
    },
    {
      fixtureId: "duplicate-row",
      inputDigest: "b".repeat(64),
      code: "duplicate_row",
      metrics: null,
      sourceRowIds: [],
      matchesExpectation: true,
    },
  ],
};
function makePort(): DeliveryKitsPort {
  return {
    list: vi.fn().mockResolvedValue([kit]),
    previewBuiltin: vi.fn().mockResolvedValue(preview),
    pickImport: vi.fn().mockResolvedValue(null),
    apply: vi.fn().mockResolvedValue({ ...kit, installed: true }),
    cancel: vi.fn().mockResolvedValue(undefined),
    previewExport: vi.fn().mockResolvedValue({ ...preview, kind: "export" }),
    saveExport: vi.fn().mockResolvedValue(true),
    runDemo: vi.fn().mockResolvedValue(demo),
  };
}
function harness(
  port: DeliveryKitsPort,
  props: Partial<ProjectDeliveryKitsPanelProps> = {},
) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  const view = (id = "project-a", revision = 0, active = true) => (
    <QueryClientProvider client={client}>
      <ProjectDeliveryKitsPanel
        projectId={id}
        projectRevision={revision}
        port={port}
        active={active}
        {...props}
      />
    </QueryClientProvider>
  );
  return { view, client };
}
describe("project delivery kits panel (fixture IPC)", () => {
  it("opens the project's bound plan after a revision change and prevents redundant binding", async () => {
    const bound = parseKitView({
      ...fixture,
      identity: {
        ...fixture.identity,
        kitId: "customer-plan",
        manifestDigest: "c".repeat(64),
      },
      manifest: {
        ...fixture.manifest,
        id: "customer-plan",
        title: "客户周报方案",
      },
      installed: true,
    });
    const port = makePort();
    vi.mocked(port.list).mockResolvedValue([kit, bound]);
    const bindDeliveryKit = vi.fn();
    const { view } = harness(port, {
      currentKit: bound.identity,
      projectAdapter: { bindDeliveryKit },
    });
    const rendered = render(view());
    await screen.findByRole("heading", { name: "客户周报方案" });
    expect(screen.getByRole("button", { name: "已用于本项目" })).toBeDisabled();
    rendered.rerender(view("project-a", 1));
    await screen.findByRole("heading", { name: "客户周报方案" });
    expect(bindDeliveryKit).not.toHaveBeenCalled();
  });
  it("keeps project identity while drafts block binding and recording", async () => {
    const port = makePort();
    vi.mocked(port.list).mockResolvedValue([{ ...kit, installed: true }]);
    const bindDeliveryKit = vi.fn();
    const recordLocalFixture = vi.fn();
    const reason = "请先保存项目资料，再更换方案或保存检查记录。";
    const { view } = harness(port, {
      currentKit: kit.identity,
      mutationBlockedReason: reason,
      projectAdapter: { bindDeliveryKit },
      evidenceAdapter: { recordLocalFixture },
    });
    render(view());
    fireEvent.click(
      await screen.findByRole("button", { name: "运行合成样例" }),
    );
    await screen.findByRole("region", { name: "样例检查结果" });
    expect(screen.getByText(reason)).toBeVisible();
    expect(screen.queryByText("请先选择项目。")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存检查记录" })).toBeDisabled();
    expect(recordLocalFixture).not.toHaveBeenCalled();
  });
  it("shows actual port results without pretending a connection or durable evidence exists", async () => {
    const port = makePort();
    const { view } = harness(port);
    render(view());
    fireEvent.click(
      await screen.findByRole("button", { name: "运行合成样例" }),
    );
    const results = await screen.findByRole("region", { name: "样例检查结果" });
    expect(within(results).getByText("180,000 元")).toBeInTheDocument();
    expect(within(results).getByText("20%")).toBeInTheDocument();
    expect(within(results).getByText(/来源行重复/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存检查记录" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "用作项目方案" })).toBeDisabled();
    expect(port.runDemo).toHaveBeenCalledWith(kit.identity);
  });
  it("previews and cancels without importing; only confirm performs a native write and reread", async () => {
    const port = makePort();
    const { view } = harness(port);
    render(view());
    fireEvent.click(await screen.findByRole("button", { name: "预览导入" }));
    const dialog = await screen.findByRole("dialog");
    expect(port.apply).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    await waitFor(() => expect(port.cancel).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "预览导入" }));
    const reopened = await screen.findByRole("dialog");
    vi.mocked(port.list).mockResolvedValue([{ ...kit, installed: true }]);
    fireEvent.click(within(reopened).getByRole("button", { name: "确认导入" }));
    await screen.findByText("已加入交付包库。尚未绑定项目或启用工具。");
    expect(port.apply).toHaveBeenCalledTimes(1);
    expect(port.list).toHaveBeenCalledTimes(2);
  });
  it("revokes a pending project-A response when project changes and does no work while hidden", async () => {
    const port = makePort();
    let resolve: (value: KitDemoResult) => void = () => undefined;
    vi.mocked(port.runDemo).mockImplementation(
      () =>
        new Promise((r) => {
          resolve = r;
        }),
    );
    const { view } = harness(port);
    const rendered = render(view());
    fireEvent.click(
      await screen.findByRole("button", { name: "运行合成样例" }),
    );
    rendered.rerender(view("project-b", 1));
    await act(async () => resolve(demo));
    expect(
      screen.queryByRole("region", { name: "样例检查结果" }),
    ).not.toBeInTheDocument();
    rendered.rerender(view("project-b", 1, false));
    const calls = vi.mocked(port.list).mock.calls.length;
    await act(async () => Promise.resolve());
    expect(port.list).toHaveBeenCalledTimes(calls);
    expect(
      screen.queryByRole("button", { name: "导入交付包" }),
    ).not.toBeInTheDocument();
  });
  it("shares an imported version only after preview confirmation and keeps cancellation and errors retryable", async () => {
    const custom = parseKitView({
      ...fixture,
      identity: { ...fixture.identity, kitId: "custom-kit" },
      manifest: { ...fixture.manifest, id: "custom-kit" },
      builtin: false,
      installed: true,
      exportable: true,
    });
    const port = makePort();
    vi.mocked(port.list).mockResolvedValue([custom]);
    vi.mocked(port.previewExport).mockResolvedValue({
      ...preview,
      kind: "export",
      kit: custom,
    });
    const { view } = harness(port);
    render(view());
    fireEvent.click(await screen.findByRole("button", { name: "分享导出" }));
    let dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/来源未验证/)).toBeInTheDocument();
    expect(
      within(dialog).getByText(/请确认内容不含客户资料或凭据/),
    ).toBeInTheDocument();
    expect(port.previewExport).toHaveBeenCalledWith(custom.identity);
    expect(port.saveExport).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    await waitFor(() => expect(port.cancel).toHaveBeenCalled());
    expect(port.saveExport).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "分享导出" }));
    dialog = await screen.findByRole("dialog");
    vi.mocked(port.saveExport)
      .mockResolvedValueOnce(false)
      .mockRejectedValueOnce(new Error("SECRET-CANARY"))
      .mockResolvedValueOnce(true);
    fireEvent.click(
      within(dialog).getByRole("button", { name: "确认并选择保存位置" }),
    );
    await waitFor(() => expect(port.saveExport).toHaveBeenCalledTimes(1));
    expect(screen.queryByText("交付包已导出。")).not.toBeInTheDocument();
    const confirm = await screen.findByRole("button", {
      name: "确认并选择保存位置",
    });
    fireEvent.click(confirm);
    await screen.findByRole("alert");
    expect(screen.queryByText(/SECRET-CANARY/)).not.toBeInTheDocument();
    fireEvent.click(
      await screen.findByRole("button", { name: "确认并选择保存位置" }),
    );
    await screen.findByText("交付包已导出。");
    expect(port.saveExport).toHaveBeenCalledTimes(3);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(port.runDemo).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "运行合成样例" })).toBeDisabled();
  });
  it("blocks conflicting import and masks unknown failures", async () => {
    const port = makePort();
    vi.mocked(port.previewBuiltin).mockResolvedValue({
      ...preview,
      conflict: true,
    });
    const { view } = harness(port);
    render(view());
    fireEvent.click(await screen.findByRole("button", { name: "预览导入" }));
    expect(
      await screen.findByRole("button", { name: "确认导入" }),
    ).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    vi.mocked(port.runDemo).mockRejectedValue(new Error("SECRET-CANARY"));
    fireEvent.click(screen.getByRole("button", { name: "运行合成样例" }));
    await screen.findByRole("alert");
    expect(screen.queryByText(/SECRET-CANARY/)).not.toBeInTheDocument();
  });
});
