import { render, screen } from "@testing-library/react";
import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SubscriptionQuotaFooter, {
  getSubscriptionExpiredHintKey,
  SubscriptionQuotaView,
} from "@/components/SubscriptionQuotaFooter";
import en from "@/i18n/locales/en.json";
import ja from "@/i18n/locales/ja.json";
import zhTW from "@/i18n/locales/zh-TW.json";
import zh from "@/i18n/locales/zh.json";
import type { SubscriptionQuota } from "@/types/subscription";

const mockUseSubscriptionQuota = vi.hoisted(() => vi.fn());

vi.mock("@/lib/query/subscription", () => ({
  useSubscriptionQuota: (...args: unknown[]) =>
    mockUseSubscriptionQuota(...args),
}));

const expiredQuota: SubscriptionQuota = {
  tool: "grok",
  credentialStatus: "expired",
  credentialMessage: "Please re-login with `grok login`.",
  success: false,
  tiers: [],
  extraUsage: null,
  error: null,
  queriedAt: Date.now(),
};

describe("getSubscriptionExpiredHintKey", () => {
  it("keeps Official Grok, xAI OAuth, and generic CLI expiry copy on separate keys", () => {
    expect(getSubscriptionExpiredHintKey("grok")).toBe(
      "subscription.grokOfficialExpiredHint",
    );
    expect(getSubscriptionExpiredHintKey("grokbuild")).toBe(
      "subscription.grokOfficialExpiredHint",
    );
    expect(getSubscriptionExpiredHintKey("xai_oauth")).toBe(
      "subscription.xaiOauthExpiredHint",
    );
    expect(getSubscriptionExpiredHintKey("claude")).toBe(
      "subscription.expiredHint",
    );
    expect(getSubscriptionExpiredHintKey("codex")).toBe(
      "subscription.expiredHint",
    );
    expect(getSubscriptionExpiredHintKey("codex_oauth")).toBe(
      "subscription.expiredHint",
    );
  });
});

describe("official and generic expiry copy", () => {
  beforeEach(() => {
    i18n.addResourceBundle("zh", "translation", zh, true, true);
    mockUseSubscriptionQuota.mockReturnValue({
      data: expiredQuota,
      isFetching: false,
      refetch: vi.fn(),
    });
  });

  afterEach(() => {
    i18n.removeResourceBundle("zh", "translation");
    i18n.addResourceBundle("zh", "translation", {});
  });

  it("points Official Grok expiry at a terminal grok login", () => {
    render(<SubscriptionQuotaFooter appId="grokbuild" isCurrent />);

    expect(screen.getByText(/grok login/)).toBeInTheDocument();
    expect(screen.queryByText(/认证中心/)).not.toBeInTheDocument();
    expect(screen.queryByText(/xai_oauth/)).not.toBeInTheDocument();
  });

  it("keeps generic CLI expiry copy for non-Grok tools", () => {
    render(
      <SubscriptionQuotaView
        quota={expiredQuota}
        loading={false}
        refetch={() => {}}
        appIdForExpiredHint="claude"
      />,
    );

    expect(screen.getByText(/请运行 claude 命令刷新登录/)).toBeInTheDocument();
    expect(screen.queryByText(/grok login/)).not.toBeInTheDocument();
    expect(screen.queryByText(/认证中心/)).not.toBeInTheDocument();
  });
});

describe("official expiry locale copy", () => {
  it("keeps the grok login literal in every locale", () => {
    for (const locale of [zh, en, ja, zhTW]) {
      expect(locale.providerForm.grokOfficialHint).toContain("grok login");
      expect(locale.subscription.grokOfficialExpiredHint).toContain(
        "grok login",
      );
      expect(locale.subscription.xaiOauthExpiredHint).not.toContain(
        "grok login",
      );
    }
  });
});
