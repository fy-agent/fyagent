import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CodexNativeModels } from "@/pages/auth/CodexNativeModels";
import type { ManagedAuthCodexNativeModels } from "@/shared/features/managed-auth";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import {
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  managedAuthOverviewFixture,
} from "../../fixtures/managedAuth";

const OTHER_OPENAI_ID = `ma1:${"6".repeat(32)}`;
const NEXT_REVISION = `mr1:${"d".repeat(64)}`;

const ready: ManagedAuthCodexNativeModels = {
  contractVersion: 1,
  accountId: OPENAI_ACCOUNT_ID,
  source: "codex_app_server",
  accountBinding: "matched",
  bindingDetail: null,
  nativeAccountId: OPENAI_ACCOUNT_ID,
  revision: CONNECTION_REVISION,
  checkedAt: "2026-10-22T04:30:00Z",
  catalogStatus: "ready",
  failure: null,
  cliVersion: "0.160.1",
  truncated: false,
  models: [
    {
      id: "gpt-6-sol",
      displayName: "GPT-6 Sol",
      isDefault: true,
      reasoningEfforts: ["low", "high"],
    },
    {
      id: "gpt-5.5",
      displayName: null,
      isDefault: false,
      reasoningEfforts: [],
    },
  ],
};

const notRead: ManagedAuthCodexNativeModels = {
  ...ready,
  nativeAccountId: null,
  catalogStatus: "not_read",
  cliVersion: null,
  models: [],
};

function setup(
  read: () => Promise<ManagedAuthCodexNativeModels>,
  revision = CONNECTION_REVISION,
) {
  const ports = createBrowserFeaturePorts();
  const getCodexNativeModels = vi.fn(read);
  ports.managedAuth.getCodexNativeModels = getCodexNativeModels;
  const accounts = [
    ...managedAuthOverviewFixture().accounts,
    {
      ...managedAuthOverviewFixture().accounts[0],
      accountId: OTHER_OPENAI_ID,
      login: "work@example.com",
      isDefault: false,
    },
  ];
  const view = (current: string) => (
    <FeatureProvider ports={ports}>
      <CodexNativeModels
        accountId={OPENAI_ACCOUNT_ID}
        revision={current}
        accounts={accounts}
        disabled={false}
      />
    </FeatureProvider>
  );
  const utils = render(view(revision));
  return {
    getCodexNativeModels,
    rerender: (current: string) => utils.rerender(view(current)),
  };
}

describe("CodexNativeModels", () => {
  it("reads only on request and shows the attributed native catalog", async () => {
    const { getCodexNativeModels } = setup(async () => ready);
    expect(getCodexNativeModels).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(
      await screen.findByText(/本机 Codex 原生官方目录 · Codex CLI 0\.160\.1/u),
    ).toBeVisible();
    expect(getCodexNativeModels).toHaveBeenCalledWith(OPENAI_ACCOUNT_ID);
    const list = screen.getByRole("list");
    expect(list).toHaveTextContent("GPT-6 Sol");
    expect(list).toHaveTextContent("gpt-6-sol");
    expect(list).toHaveTextContent("默认");
    expect(list).toHaveTextContent("gpt-5.5");
    expect(screen.getByRole("button", { name: "重新获取模型" })).toBeEnabled();
  });

  it("names the other login on a mismatch and lists nothing", async () => {
    setup(async () => ({
      ...notRead,
      accountBinding: "mismatch",
      nativeAccountId: OTHER_OPENAI_ID,
    }));
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(
      await screen.findByText(/当前登录的是「work@example\.com」/u),
    ).toBeVisible();
    expect(screen.queryByRole("list")).toBeNull();
  });

  it("explains unknown attribution and read failures by cause", async () => {
    let next: ManagedAuthCodexNativeModels = {
      ...notRead,
      accountBinding: "unknown",
      bindingDetail: "signed_out",
    };
    setup(async () => next);
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByText(/本机 Codex 尚未登录/u)).toBeVisible();

    next = {
      ...ready,
      catalogStatus: "failed",
      failure: "timed_out",
      cliVersion: null,
      models: [],
    };
    fireEvent.click(screen.getByRole("button", { name: "重新获取模型" }));
    expect(await screen.findByText(/45 秒内没有返回模型目录/u)).toBeVisible();
    expect(screen.queryByRole("list")).toBeNull();
  });

  it("drops a catalog once the connection revision moves", async () => {
    const { rerender } = setup(async () => ready);
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByRole("list")).toBeVisible();

    rerender(NEXT_REVISION);
    expect(screen.queryByRole("list")).toBeNull();
    expect(screen.getByRole("button", { name: "获取模型" })).toBeEnabled();
  });

  it("does not show a catalog read against an older revision", async () => {
    setup(async () => ready, NEXT_REVISION);
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByText(/Codex 连接状态已变化/u)).toBeVisible();
    expect(screen.queryByRole("list")).toBeNull();
  });
});
