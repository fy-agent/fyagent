import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";

const mocks = vi.hoisted(() => ({
  useCodexOauth: vi.fn(),
  renderAccountQuota: vi.fn(),
}));

vi.mock("@/components/providers/forms/hooks/useCodexOauth", () => ({
  useCodexOauth: mocks.useCodexOauth,
}));

vi.mock("@/components/CodexOauthAccountQuota", () => ({
  default: ({ accountId }: { accountId: string }) => {
    mocks.renderAccountQuota(accountId);
    return <div data-testid="account-quota">{accountId}</div>;
  },
}));

describe("CodexOAuthSection", () => {
  beforeEach(() => {
    mocks.useCodexOauth.mockReturnValue({
      accounts: [
        {
          id: "account-1",
          provider: "codex_oauth",
          login: "user@example.com",
          avatar_url: null,
          authenticated_at: 0,
          is_default: true,
          github_domain: "",
        },
      ],
      defaultAccountId: "account-1",
      hasAnyAccount: true,
      authStatus: {
        provider: "codex_oauth",
        authenticated: true,
        default_account_id: "account-1",
        accounts: [
          {
            id: "account-1",
            provider: "codex_oauth",
            login: "user@example.com",
            avatar_url: null,
            authenticated_at: 0,
            is_default: true,
            github_domain: "",
            requires_reauth: false,
            chatgpt_account_id: "ws-shared",
          },
        ],
        native_projection_available: true,
      },
    });
  });

  it("does not render account quota by default", () => {
    render(<CodexOAuthSection />);

    expect(mocks.renderAccountQuota).not.toHaveBeenCalled();
    expect(screen.queryByTestId("account-quota")).not.toBeInTheDocument();
  });

  it("renders account quota when the leftover form requests it", () => {
    render(<CodexOAuthSection showAccountQuota />);

    expect(mocks.renderAccountQuota).toHaveBeenCalledWith("account-1");
    expect(screen.getByTestId("account-quota")).toHaveTextContent("account-1");
  });

  it("leftover Auth Center is a compatibility shell without a second login owner", () => {
    render(<AuthCenterPanel />);

    expect(mocks.renderAccountQuota).not.toHaveBeenCalled();
    expect(screen.queryByTestId("account-quota")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3 })).toHaveTextContent(
      "账号与认证",
    );
  });

  it("is a picker-only leftover surface without login or remove actions", () => {
    render(
      <CodexOAuthSection
        selectedAccountId="account-1"
        onAccountSelect={vi.fn()}
      />,
    );

    expect(
      screen.getByText(/登录、重新登录和移除账号请到「账号与认证」页面完成/u),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /使用 ChatGPT 登录/u }),
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
    expect(screen.queryByText(/工作区路由 ID/u)).not.toBeInTheDocument();
    expect(screen.queryByText("ws-shared")).not.toBeInTheDocument();
  });

  it("distinguishes managed routing from native Codex projection", () => {
    mocks.useCodexOauth.mockReturnValue({
      ...mocks.useCodexOauth(),
      accounts: [
        {
          id: "account-1",
          provider: "codex_oauth",
          login: "user@example.com",
          avatar_url: null,
          authenticated_at: 0,
          is_default: true,
          github_domain: "",
          chatgpt_account_id: "ws-shared",
        },
      ],
      authStatus: {
        provider: "codex_oauth",
        authenticated: true,
        default_account_id: "account-1",
        accounts: [],
        native_projection_available: false,
      },
    });
    render(<CodexOAuthSection />);
    expect(
      screen.getByText(/Codex 当前不使用 auth\.json 保存凭据/u),
    ).toBeVisible();
    expect(screen.getByText(/这里的账号只用于 FyAgent 路由/u)).toBeVisible();
    expect(screen.queryByText(/工作区路由 ID/u)).not.toBeInTheDocument();
  });
});
