import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { ConnectionActionDialog } from "@/pages/auth/MutationDialogs";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TooltipProvider } from "@/shared/ui/primitives";
import {
  CONNECTION_PREVIEW_ID,
  OPENAI_ACCOUNT_ID,
  connectionPreviewFixture,
  managedAuthOverviewFixture,
} from "../../fixtures/managedAuth";

describe("ConnectionActionDialog", () => {
  it("resets only selection on reopen and never reuses a consumed preview", async () => {
    const user = userEvent.setup();
    const overview = managedAuthOverviewFixture();
    const otherAccountId = `ma1:${"e".repeat(32)}`;
    overview.accounts.push({
      ...overview.accounts[0],
      accountId: otherAccountId,
      login: "other@example.com",
      isDefault: false,
      connectedConsumerCount: 0,
    });
    const ports = createBrowserFeaturePorts();
    let previewId = CONNECTION_PREVIEW_ID;
    ports.managedAuth.previewConnectionAction = vi.fn(async (request) => ({
      ...connectionPreviewFixture(request),
      previewId,
    }));
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    const view = (open: boolean) => (
      <MemoryRouter>
        <TooltipProvider>
          <FeatureProvider ports={ports}>
            <ConnectionActionDialog
              connection={open ? overview.connections[0] : null}
              action={open ? "switch_account" : null}
              preferredAccountId={OPENAI_ACCOUNT_ID}
              overview={overview}
              pending={false}
              onCancel={onCancel}
              onConfirm={onConfirm}
            />
          </FeatureProvider>
        </TooltipProvider>
      </MemoryRouter>
    );
    const result = render(view(true));
    const dialog = screen.getByRole("dialog", { name: "切换 Codex 账号" });
    expect(
      within(dialog).getByRole("radio", { name: /person@example.com/ }),
    ).toBeChecked();
    await user.click(
      within(dialog).getByRole("radio", { name: /other@example.com/ }),
    );
    const confirm = within(dialog).getByRole("button", { name: "确认" });
    await waitFor(() => expect(confirm).toBeEnabled());
    act(() => {
      confirm.click();
      confirm.click();
    });
    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenLastCalledWith(
      otherAccountId,
      CONNECTION_PREVIEW_ID,
    );

    result.rerender(view(false));
    result.rerender(view(true));
    const reopened = screen.getByRole("dialog", { name: "切换 Codex 账号" });
    expect(
      within(reopened).getByRole("radio", { name: /person@example.com/ }),
    ).toBeChecked();
    await screen.findByText(
      "此次确认已使用。请关闭后重新打开，检查最新状态再操作。",
    );
    expect(
      within(reopened).getByRole("button", { name: "确认" }),
    ).toBeDisabled();
    expect(onConfirm).toHaveBeenCalledTimes(1);

    previewId = "423e4567-e89b-42d3-a456-426614174000";
    result.rerender(view(false));
    result.rerender(view(true));
    const freshConfirm = screen.getByRole("button", { name: "确认" });
    await waitFor(() => expect(freshConfirm).toBeEnabled());
    await user.click(freshConfirm);
    expect(onConfirm).toHaveBeenCalledTimes(2);
    expect(onConfirm).toHaveBeenLastCalledWith(OPENAI_ACCOUNT_ID, previewId);
  });
});
