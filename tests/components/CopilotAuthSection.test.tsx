import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CopilotAuthSection } from "@/components/providers/forms/CopilotAuthSection";

const mockUseCopilotAuth = vi.hoisted(() => vi.fn());

vi.mock("@/components/providers/forms/hooks/useCopilotAuth", () => ({
  useCopilotAuth: mockUseCopilotAuth,
}));

describe("CopilotAuthSection", () => {
  beforeEach(() => {
    mockUseCopilotAuth.mockReturnValue({
      accounts: [
        {
          id: "account-1",
          provider: "github_copilot",
          login: "octocat",
          avatar_url: null,
          authenticated_at: 0,
          is_default: true,
          github_domain: "github.com",
        },
      ],
      defaultAccountId: "account-1",
      hasAnyAccount: true,
      migrationError: null,
    });
  });

  it("is a picker-only leftover surface without login or remove actions", () => {
    render(
      <CopilotAuthSection
        selectedAccountId="account-1"
        onAccountSelect={vi.fn()}
      />,
    );

    expect(
      screen.getByText(/登录、重新登录和移除账号请到「账号与认证」页面完成/u),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /使用 GitHub 登录/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /添加其他账号/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /设为默认/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /移除账号/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /注销所有账号/u }),
    ).not.toBeInTheDocument();
  });

  it("does not render leftover migration paths or token-like diagnostics", () => {
    mockUseCopilotAuth.mockReturnValue({
      ...mockUseCopilotAuth(),
      migrationError:
        "Legacy Copilot auth migration failed: /Users/foo/copilot_auth.json gho_secret",
    });

    render(<CopilotAuthSection />);

    expect(screen.getByText(/旧认证数据迁移失败/u)).toBeVisible();
    expect(screen.queryByText(/gho_/u)).not.toBeInTheDocument();
    expect(screen.queryByText(/copilot_auth\.json/u)).not.toBeInTheDocument();
    expect(screen.queryByText(/\/Users\/foo/u)).not.toBeInTheDocument();
  });
});
