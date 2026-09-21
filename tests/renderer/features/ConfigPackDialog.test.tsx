import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import fixture from "../../fixtures/configPackDtoContract.v1.json";
import { parseImportPreview } from "@/domain/config-pack";
import type { ConfigPackPort } from "@/shared/features/config-pack";
import { ConfigPackDialog } from "@/shared/features/config-pack-ui/ConfigPackDialog";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const preview = parseImportPreview({
  previewId: "11111111-1111-4111-8111-111111111111",
  digest: "a".repeat(64),
  entries: fixture.providers.map((provider) => ({
    provider,
    action: "add",
    conflict: false,
    canOverwrite: false,
    existing: null,
    credentialsRequired: true,
  })),
});
function setup(candidate = preview.entries[0].provider) {
  const port: ConfigPackPort = {
    list: vi.fn().mockResolvedValue({
      entries: [{ selectionId: "a".repeat(64), provider: candidate }],
      excluded: 1,
    }),
    pickFile: vi.fn().mockResolvedValue(JSON.stringify(fixture)),
    previewImport: vi.fn().mockResolvedValue(preview),
    apply: vi
      .fn()
      .mockResolvedValue({ providers: fixture.providers, skipped: 0 }),
    previewExport: vi.fn().mockResolvedValue({
      exportId: preview.previewId,
      digest: preview.digest,
      text: JSON.stringify(fixture),
    }),
    saveExport: vi.fn().mockResolvedValue(false),
    cancel: vi.fn().mockResolvedValue(undefined),
  };
  const ports = createBrowserFeaturePorts();
  ports.configPack = port;
  const close = vi.fn();
  const fill = vi.fn();
  const view = render(
    <FeatureProvider ports={ports}>
      <ConfigPackDialog
        open
        originRef={{ current: null }}
        onClose={close}
        onFillModelForm={fill}
      />
    </FeatureProvider>,
  );
  return { port, close, fill, ...view };
}
async function importTab() {
  const tab = screen.getByRole("tab", { name: "导入" });
  fireEvent.mouseDown(tab, { button: 0, ctrlKey: false });
  fireEvent.click(tab);
  await screen.findByRole("button", { name: "选择 JSON 文件" });
}
describe("configuration pack dialog with fixture IPC", () => {
  it("exports selected entries and does not treat picker cancel as success", async () => {
    const { port, fill, close } = setup();
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "选择 Team Codex · Codex" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "生成导出内容" }));
    const save = await screen.findByRole("button", { name: "另存为文件" });
    expect(port.previewExport).toHaveBeenCalledWith(["a".repeat(64)]);
    expect(screen.getByLabelText("导出文本（可选择复制）")).toHaveValue(
      JSON.stringify(fixture),
    );
    fireEvent.click(save);
    await waitFor(() => expect(port.saveExport).toHaveBeenCalledTimes(1));
    expect(screen.queryByText("配置文件已保存。")).not.toBeInTheDocument();
    expect(fill).not.toHaveBeenCalled();
    const fillButton = screen.getByRole("button", {
      name: "将 Team Codex 填入模型配置表单",
    });
    await waitFor(() => expect(fillButton).toBeEnabled());
    await act(async () => {
      fireEvent.click(fillButton);
    });
    expect(fill).toHaveBeenCalledWith(fixture.providers[0]);
    expect(close).toHaveBeenCalledTimes(1);
    expect(port.apply).not.toHaveBeenCalled();
  });
  it("previews actual fields, revokes changes and saves once only after renewed preview", async () => {
    const { port, fill, close } = setup();
    await importTab();
    fireEvent.click(screen.getByRole("button", { name: "选择 JSON 文件" }));
    await waitFor(() =>
      expect(screen.getByLabelText("或粘贴配置文本（最多 64 KB）")).toHaveValue(
        JSON.stringify(fixture),
      ),
    );
    expect(port.apply).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "预览导入" }));
    await screen.findByText("将保存的字段");
    expect(screen.getByText("https://api.example.com/v1")).toBeVisible();
    fireEvent.change(
      screen.getByRole("combobox", { name: "处理 Team Codex" }),
      { target: { value: "skip" } },
    );
    expect(screen.getByRole("button", { name: "确认保存" })).toBeDisabled();
    const changed = {
      ...preview,
      entries: [
        { ...preview.entries[0], action: "skip" as const },
        preview.entries[1],
      ],
    };
    vi.mocked(port.previewImport).mockResolvedValue(changed);
    vi.mocked(port.apply).mockResolvedValue({
      providers: [preview.entries[1].provider],
      skipped: 1,
    });
    fireEvent.click(screen.getByRole("button", { name: "更新预览" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "确认保存" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "确认保存" }));
    fireEvent.click(screen.getByRole("button", { name: "确认保存" }));
    await screen.findByText(/已保存 1 项未启用连接，跳过 1 项/);
    expect(port.apply).toHaveBeenCalledTimes(1);
    expect(port.apply).toHaveBeenCalledWith(changed);
    expect(fill).not.toHaveBeenCalled();
    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "将 Team Claude 填入模型配置表单" }),
      );
    });
    expect(fill).toHaveBeenCalledTimes(1);
    expect(fill).toHaveBeenCalledWith(preview.entries[1].provider);
    expect(close).toHaveBeenCalledTimes(1);
    expect(port.apply).toHaveBeenCalledTimes(1);
  });
  it("reopens stored fields in the model form without losing Chat protocol or writing", async () => {
    const candidate = {
      ...preview.entries[0].provider,
      wireApi: "chat" as const,
    };
    const { port, fill, close } = setup(candidate);
    const fillButton = await screen.findByRole("button", {
      name: "将 Team Codex 填入模型配置表单",
    });
    await act(async () => {
      fireEvent.click(fillButton);
    });
    expect(fill).toHaveBeenCalledWith(candidate);
    expect(close).toHaveBeenCalledTimes(1);
    expect(port.apply).not.toHaveBeenCalled();
    expect(port.saveExport).not.toHaveBeenCalled();
    expect(port.previewImport).not.toHaveBeenCalled();
  });
  it("blocks overwrite of protected connections and invalidates text edits", async () => {
    const { port } = setup();
    const conflict = {
      ...preview,
      entries: [
        { ...preview.entries[0], action: "skip" as const, conflict: true },
      ],
    };
    vi.mocked(port.previewImport).mockResolvedValue(conflict);
    await importTab();
    fireEvent.change(screen.getByLabelText("或粘贴配置文本（最多 64 KB）"), {
      target: { value: JSON.stringify(fixture) },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览导入" }));
    await screen.findByText("将保存的字段");
    expect(
      screen.queryByRole("option", { name: "覆盖未启用草稿" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认保存" })).toBeDisabled();
    fireEvent.change(
      screen.getByRole("combobox", { name: "处理 Team Codex" }),
      { target: { value: "rename" } },
    );
    expect(screen.getByLabelText("另存 Team Codex")).toHaveValue(
      "Team Codex 副本",
    );
    fireEvent.change(screen.getByLabelText("或粘贴配置文本（最多 64 KB）"), {
      target: { value: "{}" },
    });
    expect(screen.queryByText("将保存的字段")).not.toBeInTheDocument();
    expect(port.apply).not.toHaveBeenCalled();
  });
  it("discards a late preview after close and redacts unknown failures", async () => {
    const first = setup();
    await importTab();
    fireEvent.change(screen.getByLabelText("或粘贴配置文本（最多 64 KB）"), {
      target: { value: JSON.stringify(fixture) },
    });
    let resolve: (p: typeof preview) => void = () => undefined;
    vi.mocked(first.port.previewImport).mockImplementation(
      () =>
        new Promise((r) => {
          resolve = r;
        }),
    );
    fireEvent.click(screen.getByRole("button", { name: "预览导入" }));
    first.unmount();
    await act(async () => resolve(preview));
    expect(first.port.cancel).toHaveBeenCalledWith(preview.previewId);
    expect(first.port.apply).not.toHaveBeenCalled();
    const next = setup();
    vi.mocked(next.port.previewExport).mockRejectedValue(
      new Error("SECRET-CANARY"),
    );
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "选择 Team Codex · Codex" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "生成导出内容" }));
    await screen.findByRole("alert");
    expect(screen.queryByText(/SECRET-CANARY/)).not.toBeInTheDocument();
  });
});
