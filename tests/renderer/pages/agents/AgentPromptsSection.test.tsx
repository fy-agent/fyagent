import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AgentPromptsSection } from "@/pages/agents/AgentPromptsSection";
import {
  PRODUCT_DIRECTORY,
  type ProductDirectoryEntry,
} from "@/shared/features/directory";
import { FeatureProvider } from "@/shared/features/provider";
import type { ManagedPrompt, PromptAppId } from "@/shared/features/types";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const source = PRODUCT_DIRECTORY.find((entry) => entry.agentId === "codex")!;
const target = PRODUCT_DIRECTORY.find(
  (entry) => entry.agentId === "claude-code",
)!;

function deferred() {
  let resolve!: () => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<void>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function fixture() {
  const ports = createBrowserFeaturePorts();
  const stores: Partial<Record<PromptAppId, ManagedPrompt[]>> = {
    codex: [
      {
        id: "first",
        name: "Source first",
        content: "source first body",
        enabled: false,
      },
      {
        id: "shared",
        name: "Source second",
        content: "source second body",
        enabled: false,
      },
    ],
    claude: [
      {
        id: "shared",
        name: "Target first",
        content: "target first body",
        enabled: false,
      },
      {
        id: "first",
        name: "Target second",
        content: "target second body",
        enabled: false,
      },
    ],
  };
  ports.prompts.getAll = vi.fn(async (app: PromptAppId) =>
    (stores[app] ?? []).map((row) => ({ ...row })),
  );
  ports.prompts.enable = vi.fn(async (app: PromptAppId, id: string) => {
    stores[app] = stores[app]?.map((row) => ({
      ...row,
      enabled: row.id === id,
    }));
  });
  const view = (entry: ProductDirectoryEntry) => (
    <FeatureProvider ports={ports}>
      <AgentPromptsSection entry={entry} onOpenManagement={() => undefined} />
    </FeatureProvider>
  );
  const rendered = render(view(source));
  return {
    ports,
    stores,
    switchTo: (entry: ProductDirectoryEntry) => rendered.rerender(view(entry)),
  };
}

describe("AgentPromptsSection app lifecycle #364", () => {
  it("resets search, selection and feedback on A→B→A but preserves same-app state", async () => {
    const user = userEvent.setup();
    const { ports, switchTo } = fixture();
    await user.click(
      await screen.findByRole("button", { name: /Source second/ }),
    );
    await user.click(screen.getByRole("button", { name: "启用" }));
    expect(
      await screen.findByText("已在 Codex 中启用此提示词。"),
    ).toBeVisible();
    await user.type(screen.getByRole("searchbox"), "Source");
    switchTo({ ...source });
    expect(screen.getByRole("searchbox")).toHaveValue("Source");
    expect(
      screen.getByRole("heading", { name: "Source second" }),
    ).toBeVisible();
    switchTo(target);
    expect(
      await screen.findByRole("heading", { name: "Target first" }),
    ).toBeVisible();
    expect(screen.getByRole("searchbox")).toHaveValue("");
    expect(screen.getByText("target first body")).toBeVisible();
    expect(
      screen.queryByText("已在 Codex 中启用此提示词。"),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "启用" })).toBeEnabled();
    switchTo(source);
    expect(
      await screen.findByRole("heading", { name: "Source first" }),
    ).toBeVisible();
    expect(screen.getByRole("searchbox")).toHaveValue("");
    expect(
      screen.queryByText("已在 Codex 中启用此提示词。"),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Source second/ }));
    expect(screen.getByRole("button", { name: "当前已启用" })).toBeDisabled();
    expect(ports.prompts.enable).toHaveBeenCalledExactlyOnceWith(
      "codex",
      "shared",
    );
  });

  it.each(["success", "failure", "mismatch"] as const)(
    "isolates late source %s from the target with colliding IDs",
    async (outcome) => {
      const user = userEvent.setup();
      const { ports, stores, switchTo } = fixture();
      const pending = deferred();
      let completed = false;
      let writeSettled = false;
      ports.prompts.enable = vi.fn(async (app: PromptAppId, id: string) => {
        try {
          await pending.promise;
          if (outcome === "success")
            stores[app] = stores[app]?.map((row) => ({
              ...row,
              enabled: row.id === id,
            }));
        } finally {
          writeSettled = true;
        }
      });
      const getAll = ports.prompts.getAll;
      ports.prompts.getAll = vi.fn(async (app: PromptAppId) => {
        const rows = await getAll(app);
        if (writeSettled) completed = true;
        return rows;
      });
      await user.click(
        await screen.findByRole("button", { name: /Source second/ }),
      );
      await user.click(screen.getByRole("button", { name: "启用" }));
      expect(screen.getByRole("button", { name: "启用" })).toBeDisabled();
      switchTo(target);
      expect(
        await screen.findByRole("heading", { name: "Target first" }),
      ).toBeVisible();
      await user.type(screen.getByRole("searchbox"), "Target");
      await act(async () => {
        if (outcome === "failure") pending.reject(new Error("source failed"));
        else pending.resolve();
      });
      await waitFor(() => expect(completed).toBe(true));
      expect(screen.getByRole("searchbox")).toHaveValue("Target");
      expect(
        screen.getByRole("heading", { name: "Target first" }),
      ).toBeVisible();
      expect(screen.getByText("target first body")).toBeVisible();
      expect(screen.getByRole("button", { name: "启用" })).toBeEnabled();
      expect(
        screen.queryByText(/已在 Codex 中启用|无法确认提示词是否已启用/),
      ).not.toBeInTheDocument();
      expect(stores.claude?.every((row) => !row.enabled)).toBe(true);
      expect(ports.prompts.enable).toHaveBeenCalledExactlyOnceWith(
        "codex",
        "shared",
      );
    },
  );

  it("lets the target enable a colliding ID while the source write is pending", async () => {
    const user = userEvent.setup();
    const { ports, stores, switchTo } = fixture();
    const pending = deferred();
    const enable = ports.prompts.enable;
    ports.prompts.enable = vi.fn(async (app: PromptAppId, id: string) => {
      if (app === "codex") await pending.promise;
      await enable(app, id);
    });
    await user.click(
      await screen.findByRole("button", { name: /Source second/ }),
    );
    await user.click(screen.getByRole("button", { name: "启用" }));
    switchTo(target);
    await screen.findByRole("heading", { name: "Target first" });
    expect(screen.getByRole("button", { name: "启用" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "启用" }));
    expect(
      await screen.findByText("已在 Claude Code 中启用此提示词。"),
    ).toBeVisible();
    expect(stores.claude?.find((row) => row.id === "shared")?.enabled).toBe(
      true,
    );
    await act(async () => {
      pending.resolve();
    });
    await waitFor(() =>
      expect(stores.codex?.find((row) => row.id === "shared")?.enabled).toBe(
        true,
      ),
    );
    expect(screen.getByText("已在 Claude Code 中启用此提示词。")).toBeVisible();
    expect(
      screen.queryByText("已在 Codex 中启用此提示词。"),
    ).not.toBeInTheDocument();
    expect(ports.prompts.enable).toHaveBeenNthCalledWith(1, "codex", "shared");
    expect(ports.prompts.enable).toHaveBeenNthCalledWith(2, "claude", "shared");
  });

  it("does not read or write for an unsupported app", async () => {
    const { ports, switchTo } = fixture();
    await screen.findByRole("heading", { name: "Source first" });
    ports.prompts.getAll = vi.fn();
    switchTo(PRODUCT_DIRECTORY[0]);
    expect(screen.getByText("此应用暂不支持提示词管理")).toBeVisible();
    expect(ports.prompts.getAll).not.toHaveBeenCalled();
    expect(ports.prompts.enable).not.toHaveBeenCalled();
  });
});
