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
      pollingState: "idle",
      deviceCode: null,
      error: null,
      isPolling: false,
      isAddingAccount: false,
      isRemovingAccount: false,
      isSettingDefaultAccount: false,
      addAccount: vi.fn(),
      removeAccount: vi.fn(),
      setDefaultAccount: vi.fn(),
      cancelAuth: vi.fn(),
      logout: vi.fn(),
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
    expect(screen.getByText(/工作区路由 ID/u)).toBeVisible();
    expect(screen.getByText(/ws-shared/u)).toBeVisible();
  });
});
