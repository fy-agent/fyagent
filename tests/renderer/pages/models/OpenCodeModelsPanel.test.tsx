import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { OpenCodeModelsPanel } from "@/pages/models/OpenCodeModelsPanel";
import type {
  FetchedModelList,
  OpenCodeModelSnapshot,
} from "@/shared/features/models";
import type { FeaturePorts } from "@/shared/features/ports";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";

const snapshot: OpenCodeModelSnapshot = {
  providers: [
    {
      id: "vibekey-exact",
      name: "VibeKey",
      editable: true,
      modelIds: ["grok-4.6"],
    },
    {
      id: "other-provider",
      name: "Other",
      editable: true,
      modelIds: ["other-model"],
    },
  ],
  revision: "revision-1",
  selectedModel: "vibekey-exact/grok-4.6",
  path: "~/.config/opencode/opencode.json",
  backupPath: "~/.config/opencode/opencode.json.backup",
  exists: true,
};
const fetched: FetchedModelList = {
  models: [
    { id: "grok-4.6" },
    { id: "grok-4.7", ownedBy: "xai" },
    { id: "unselected-model" },
  ],
  truncated: false,
};
function setupPorts() {
  const ports = createBrowserFeaturePorts();
  ports.opencodeModels.getSnapshot = vi.fn(async () => snapshot);
  ports.opencodeModels.fetchProviderModels = vi.fn(async () => fetched);
  ports.opencodeModels.saveModels = vi.fn(async () => ({
    state: "saved" as const,
    revision: "revision-2",
    modelCount: 2,
    createdEntries: 1,
    updatedEntries: 0,
  }));
  return ports;
}
function renderPanel(ports: FeaturePorts) {
  const onBlockWrites = vi.fn();
  const rendered = render(
    <StrictMode>
      <MemoryRouter>
        <TooltipProvider delayDuration={0} skipDelayDuration={0}>
          <FeatureProvider ports={ports}>
            <OpenCodeModelsPanel
              active
              writesBlocked={false}
              onBlockWrites={onBlockWrites}
            />
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    </StrictMode>,
  );
  return { ...rendered, onBlockWrites };
}
async function ready() {
  await waitFor(() =>
    expect(screen.getByLabelText("供应商")).toHaveValue(
      "existing:vibekey-exact",
    ),
  );
}
async function choose(
  user: ReturnType<typeof userEvent.setup>,
  id = "grok-4.7",
) {
  await user.click(
    within(screen.getByRole("region", { name: "可添加的模型 ID" })).getByRole(
      "button",
      { name: id },
    ),
  );
}
async function save(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "保存并应用" }));
  await user.click(
    within(await screen.findByRole("dialog", { name: "保存前确认" })).getByRole(
      "button",
      { name: "确认保存" },
    ),
  );
}

describe("OpenCode saved provider model refresh", () => {
  it("reuses saved connection identity, requires a selection, and rereads the saved model list", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    let current = snapshot;
    ports.opencodeModels.getSnapshot = vi.fn(async () => current);
    ports.opencodeModels.saveModels = vi.fn(async () => {
      current = {
        ...snapshot,
        revision: "revision-2",
        providers: snapshot.providers.map((provider) =>
          provider.id === "vibekey-exact"
            ? { ...provider, modelIds: ["grok-4.6", "grok-4.7"] }
            : provider,
        ),
      };
      return {
        state: "saved" as const,
        revision: "revision-2",
        modelCount: 2,
        createdEntries: 1,
        updatedEntries: 0,
      };
    });
    renderPanel(ports);
    await ready();
    expect(screen.getByLabelText("服务地址")).toHaveValue("");
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(
      ports.opencodeModels.fetchProviderModels,
    ).toHaveBeenCalledExactlyOnceWith({
      providerId: "vibekey-exact",
      expectedRevision: "revision-1",
      baseUrl: "",
      apiKey: "",
      allowNoApiKey: false,
    });
    const candidates = await screen.findByRole("region", {
      name: "可添加的模型 ID",
    });
    expect(
      within(candidates).getByRole("button", { name: "grok-4.7" }),
    ).toHaveAttribute("aria-pressed", "false");
    expect(
      within(candidates).queryByRole("button", { name: "grok-4.6" }),
    ).toBeNull();
    await user.click(screen.getByRole("button", { name: "保存并应用" }));
    expect(await screen.findByText("请至少添加一个模型 ID")).toBeVisible();
    expect(ports.opencodeModels.saveModels).not.toHaveBeenCalled();
    await choose(user);
    expect(
      within(candidates).getByRole("button", { name: "grok-4.7" }),
    ).toHaveAttribute("aria-pressed", "true");
    await save(user);
    expect(ports.opencodeModels.saveModels).toHaveBeenCalledExactlyOnceWith({
      providerId: "vibekey-exact",
      providerName: "VibeKey",
      baseUrl: "",
      apiKey: "",
      selectedModelIds: ["grok-4.7"],
      removedModelIds: [],
      expectedRevision: "revision-1",
    });
    expect(await screen.findByText("OpenCode 模型配置已保存")).toBeVisible();
    expect(ports.opencodeModels.getSnapshot).toHaveBeenCalledTimes(2);
    await user.click(
      screen.getByRole("button", { name: /当前已有的第三方模型 ID/ }),
    );
    expect(
      within(screen.getByTestId("opencode-model-ids")).getByText("grok-4.7"),
    ).toBeVisible();
    expect(
      within(screen.getByTestId("opencode-model-ids")).getByText("grok-4.6"),
    ).toBeVisible();
    expect(current.selectedModel).toBe("vibekey-exact/grok-4.6");
  });

  it("returns to saved connection reuse after confirming an explicit connection replacement", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    let current = snapshot;
    ports.opencodeModels.getSnapshot = vi.fn(async () => current);
    ports.opencodeModels.saveModels = vi.fn(async () => {
      current = { ...snapshot, revision: "revision-2" };
      return {
        state: "saved" as const,
        revision: "revision-2",
        modelCount: 2,
        createdEntries: 1,
        updatedEntries: 0,
      };
    });
    renderPanel(ports);
    await ready();
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://replacement.test/v1",
    );
    await user.type(screen.getByLabelText("API Key"), "replacement-key");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    await choose(user);
    await save(user);
    expect(await screen.findByText("OpenCode 模型配置已保存")).toBeVisible();
    expect(ports.opencodeModels.saveModels).toHaveBeenCalledWith(
      expect.objectContaining({
        baseUrl: "https://replacement.test/v1",
        apiKey: "replacement-key",
      }),
    );
    expect(screen.getByLabelText("服务地址")).toHaveValue("");
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(ports.opencodeModels.fetchProviderModels).toHaveBeenLastCalledWith({
      providerId: "vibekey-exact",
      expectedRevision: "revision-2",
      baseUrl: "",
      apiKey: "",
      allowNoApiKey: false,
    });
  });

  it("retains explicit selections and manual IDs across refreshes and a failed fetch", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    renderPanel(ports);
    await ready();
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    await choose(user);
    await user.type(screen.getByLabelText("自定义模型 ID"), "manual-model");
    await user.click(screen.getByRole("button", { name: "填入" }));
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(
      within(screen.getByRole("region", { name: "可添加的模型 ID" })).getByRole(
        "button",
        { name: "grok-4.7" },
      ),
    ).toHaveAttribute("aria-pressed", "true");
    await choose(user);
    ports.opencodeModels.fetchProviderModels = vi.fn(async () => {
      throw new Error("PRIVATE-UPSTREAM-DETAIL");
    });
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("模型读取失败")).toBeVisible();
    expect(document.body.textContent).not.toContain("PRIVATE-UPSTREAM-DETAIL");
    await save(user);
    expect(ports.opencodeModels.saveModels).toHaveBeenCalledWith(
      expect.objectContaining({
        selectedModelIds: ["manual-model"],
        removedModelIds: [],
      }),
    );
    await user.click(
      screen.getByRole("button", { name: /当前已有的第三方模型 ID/ }),
    );
    expect(
      within(screen.getByTestId("opencode-model-ids")).getByText("grok-4.6"),
    ).toBeVisible();
  });

  it("rereads a stale saved connection while preserving the current draft", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    ports.opencodeModels.fetchProviderModels = vi.fn(async () => {
      throw { code: "OPENCODE_CONCURRENT_MODIFICATION" };
    });
    renderPanel(ports);
    await ready();
    await user.type(screen.getByLabelText("自定义模型 ID"), "manual-model");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("配置已被其他操作修改")).toBeVisible();
    expect(ports.opencodeModels.getSnapshot).toHaveBeenCalledTimes(2);
    expect(screen.getByLabelText("自定义模型 ID")).toHaveValue("manual-model");
    expect(
      screen.queryByRole("region", { name: "可添加的模型 ID" }),
    ).toBeNull();
  });

  it("rejects obsolete fetch results after editing and clears candidates on provider switches", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    let resolveFetch!: (value: FetchedModelList) => void;
    ports.opencodeModels.fetchProviderModels = vi.fn(
      () =>
        new Promise<FetchedModelList>((resolve) => {
          resolveFetch = resolve;
        }),
    );
    renderPanel(ports);
    await ready();
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(screen.getByLabelText("供应商")).toBeDisabled();
    fireEvent.change(screen.getByLabelText("自定义模型 ID"), {
      target: { value: "manual-model" },
    });
    await act(async () => {
      resolveFetch(fetched);
    });
    expect(
      screen.queryByRole("region", { name: "可添加的模型 ID" }),
    ).toBeNull();
    expect(screen.getByLabelText("自定义模型 ID")).toHaveValue("manual-model");
    ports.opencodeModels.fetchProviderModels = vi.fn(async () => fetched);
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    await choose(user);
    await user.selectOptions(
      screen.getByLabelText("供应商"),
      "existing:other-provider",
    );
    await user.click(
      await screen.findByRole("button", { name: "放弃修改并切换" }),
    );
    expect(
      screen.queryByRole("region", { name: "可添加的模型 ID" }),
    ).toBeNull();
    expect(screen.getByLabelText("自定义模型 ID")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(ports.opencodeModels.fetchProviderModels).toHaveBeenLastCalledWith({
      providerId: "other-provider",
      expectedRevision: "revision-1",
      baseUrl: "",
      apiKey: "",
      allowNoApiKey: false,
    });
    expect(
      within(screen.getByRole("region", { name: "可添加的模型 ID" })).getByRole(
        "button",
        { name: "grok-4.7" },
      ),
    ).toHaveAttribute("aria-pressed", "false");
  });

  it("requires a replacement key for an explicit address and keeps new-provider validation", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    renderPanel(ports);
    await ready();
    await user.type(
      screen.getByLabelText("服务地址"),
      "https://replacement.test/v1",
    );
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(
      await screen.findByText("填写服务地址时，请同时填写新的 API Key"),
    ).toBeVisible();
    expect(ports.opencodeModels.fetchProviderModels).not.toHaveBeenCalled();
    await user.selectOptions(screen.getByLabelText("供应商"), "new");
    await user.click(
      await screen.findByRole("button", { name: "放弃修改并切换" }),
    );
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("请输入有效的服务地址")).toBeVisible();
    await user.type(
      screen.getByLabelText(/^服务地址/),
      "https://new-provider.test/v1",
    );
    await user.click(screen.getByRole("button", { name: "拉取模型" }));
    expect(await screen.findByText("请输入 API Key")).toBeVisible();
    expect(ports.opencodeModels.fetchProviderModels).not.toHaveBeenCalled();
  });

  it("does not claim success or clear the draft when save reread fails", async () => {
    const user = userEvent.setup();
    const ports = setupPorts();
    const { onBlockWrites } = renderPanel(ports);
    await ready();
    await user.type(screen.getByLabelText("自定义模型 ID"), "manual-model");
    ports.opencodeModels.getSnapshot = vi.fn(async () => {
      throw new Error("unavailable");
    });
    await save(user);
    expect(await screen.findByText("暂时无法确认模型配置")).toBeVisible();
    expect(screen.queryByText("OpenCode 模型配置已保存")).toBeNull();
    expect(onBlockWrites).toHaveBeenCalledOnce();
    expect(
      within(screen.getByTestId("opencode-draft-models")).getByText(
        "manual-model",
      ),
    ).toBeVisible();
  });
});
