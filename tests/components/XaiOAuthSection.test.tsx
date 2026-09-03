import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { XaiOAuthSection } from "@/components/providers/forms/XaiOAuthSection";

const mockUseXaiOauth = vi.hoisted(() => vi.fn());

vi.mock("@/components/providers/forms/hooks/useXaiOauth", () => ({
  useXaiOauth: mockUseXaiOauth,
}));

describe("XaiOAuthSection", () => {
  beforeEach(() => {
    mockUseXaiOauth.mockReturnValue({
      accounts: [
        {
          id: "expired-account",
          login: "expired@example.com",
          avatar_url: null,
          authenticated_at: 1,
          github_domain: "x.ai",
          requires_reauth: true,
        },
        {
          id: "usable-account",
          login: "usable@example.com",
          avatar_url: null,
          authenticated_at: 2,
          github_domain: "x.ai",
          requires_reauth: false,
        },
      ],
      defaultAccountId: "usable-account",
      hasAnyAccount: true,
      isAuthenticated: true,
    });
  });

  it("keeps a selected account visible when it requires reauthentication", () => {
    render(
      <XaiOAuthSection
        selectedAccountId="expired-account"
        onAccountSelect={vi.fn()}
      />,
    );

    expect(screen.getByRole("combobox")).toHaveTextContent(
      "expired@example.com",
    );
    expect(screen.getByRole("combobox")).toHaveTextContent("凭据已失效");
  });

  it("is a picker-only leftover surface without login or remove actions", () => {
    render(
      <XaiOAuthSection
        selectedAccountId="usable-account"
        onAccountSelect={vi.fn()}
      />,
    );

    expect(
      screen.getByText(/登录、重新登录和移除账号请到「账号与认证」页面完成/u),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /使用 xAI 登录/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /添加账号或重新登录/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /设为默认/u }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /移除账号/u }),
    ).not.toBeInTheDocument();
  });
});
