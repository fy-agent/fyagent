import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { AuthPage } from "@/pages/auth/Page";
import {
  parseManagedAuthLoginSession,
  type ManagedAuthPort,
} from "@/shared/features/managed-auth";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import {
  deviceLoginSessionFixture,
  managedAuthOverviewFixture,
} from "../../fixtures/managedAuth";

describe("managed login startup feedback", () => {
  it.each([
    {
      label: "SecretUnavailable",
      fail: () => {
        throw { contractVersion: 1, reasonCode: "secret_unavailable" };
      },
      expected: "系统凭据库暂时不可用。",
    },
    {
      label: "OperationConflict",
      fail: () => {
        throw { contractVersion: 1, reasonCode: "operation_conflict" };
      },
      expected: "已有账号操作正在进行，请先完成或取消。",
    },
    {
      label: "an invalid native response",
      fail: () =>
        parseManagedAuthLoginSession({
          ...deviceLoginSessionFixture(),
          sessionId: "private-token C:/private/auth.json",
        }),
      expected: "请稍后重试。",
    },
  ])(
    "shows $label without a session and preserves retry, choices and exit focus",
    async ({ fail, expected }) => {
      const user = userEvent.setup();
      const ports = createBrowserFeaturePorts();
      const startLogin = vi
        .fn<ManagedAuthPort["startLogin"]>()
        .mockImplementationOnce(async () => {
          fail();
          throw new Error("The startup failure fixture must reject");
        })
        .mockImplementationOnce(async () =>
          parseManagedAuthLoginSession(deviceLoginSessionFixture()),
        );
      ports.managedAuth.getOverview = vi.fn(async () =>
        managedAuthOverviewFixture(),
      );
      ports.managedAuth.startLogin = startLogin;
      ports.managedAuth.getLoginSession = vi.fn(async () =>
        deviceLoginSessionFixture(),
      );
      ports.managedAuth.cancelLogin = vi.fn();
      render(
        <MemoryRouter
          initialEntries={["/auth?consumer=codex&view=connections"]}
        >
          <TooltipProvider delayDuration={0} skipDelayDuration={0}>
            <FeatureProvider ports={ports}>
              <AuthPage />
            </FeatureProvider>
          </TooltipProvider>
        </MemoryRouter>,
      );

      const origin = await screen.findByRole("button", { name: "添加账号" });
      await user.click(origin);
      let dialog = screen.getByRole("dialog", { name: "添加官方账号" });
      await user.click(
        within(dialog).getByRole("radio", { name: "设备码登录" }),
      );
      await user.click(within(dialog).getByRole("button", { name: "下一步" }));
      const continueButton = within(dialog).getByRole("button", {
        name: "继续",
      });
      await user.click(continueButton);

      expect(await within(dialog).findByText(expected)).toBeVisible();
      expect(within(dialog).getAllByText(expected)).toHaveLength(1);
      expect(continueButton).toBeEnabled();
      expect(continueButton).toHaveFocus();
      expect(
        within(dialog).queryByRole("button", { name: "取消登录" }),
      ).not.toBeInTheDocument();
      expect(dialog.textContent).not.toContain("private-token");
      expect(dialog.textContent).not.toContain("C:/private");

      await user.click(within(dialog).getByRole("button", { name: "上一步" }));
      expect(
        within(dialog).getByRole("radio", { name: "设备码登录" }),
      ).toBeChecked();
      expect(
        within(dialog).getByRole("radio", { name: "连接 Codex" }),
      ).toBeChecked();
      await user.keyboard("{Escape}");
      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
      await waitFor(() => expect(origin).toHaveFocus());
      expect(ports.managedAuth.cancelLogin).not.toHaveBeenCalled();

      await user.click(origin);
      dialog = screen.getByRole("dialog", { name: "添加官方账号" });
      expect(
        within(dialog).getByRole("radio", { name: "设备码登录" }),
      ).toBeChecked();
      expect(
        within(dialog).getByRole("radio", { name: "连接 Codex" }),
      ).toBeChecked();
      await user.click(within(dialog).getByRole("button", { name: "下一步" }));
      await user.click(within(dialog).getByRole("button", { name: "继续" }));
      expect(startLogin).toHaveBeenCalledTimes(2);
      expect(startLogin).toHaveBeenLastCalledWith({
        provider: "openai",
        purpose: "connect_consumer",
        consumer: "codex",
        method: "device_code",
        accountId: null,
      });
      expect(await within(dialog).findByText("ABCD-EFGH")).toBeVisible();
      expect(within(dialog).queryByText(expected)).not.toBeInTheDocument();

      await user.keyboard("{Escape}");
      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
      expect(ports.managedAuth.cancelLogin).not.toHaveBeenCalled();
    },
  );
});
