import { render, screen } from "@testing-library/react";
import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SubscriptionQuotaView } from "@/components/SubscriptionQuotaFooter";
import XaiOauthQuotaFooter from "@/components/XaiOauthQuotaFooter";
import en from "@/i18n/locales/en.json";
import ja from "@/i18n/locales/ja.json";
import zhTW from "@/i18n/locales/zh-TW.json";
import zh from "@/i18n/locales/zh.json";
import type { SubscriptionQuota } from "@/types/subscription";

const mockUseXaiOauthQuota = vi.hoisted(() => vi.fn());

vi.mock("@/lib/query/subscription", () => ({
  useXaiOauthQuota: (...args: unknown[]) => mockUseXaiOauthQuota(...args),
}));

const expiredQuota: SubscriptionQuota = {
  tool: "xai_oauth",
  credentialStatus: "expired",
  credentialMessage: null,
  success: false,
  tiers: [],
  extraUsage: null,
  error: null,
  queriedAt: Date.now(),
};

describe("XaiOauthQuotaFooter expiry copy", () => {
  beforeEach(() => {
    i18n.addResourceBundle("zh", "translation", zh, true, true);
    mockUseXaiOauthQuota.mockReturnValue({
      data: expiredQuota,
      isFetching: false,
      refetch: vi.fn(),
    });
  });

  afterEach(() => {
    i18n.removeResourceBundle("zh", "translation");
    i18n.addResourceBundle("zh", "translation", {});
  });

  it("points expired xAI OAuth credentials at Auth Center", () => {
    render(<XaiOauthQuotaFooter isCurrent />);

    expect(screen.getByText(/认证中心/)).toBeInTheDocument();
    expect(screen.queryByText(/grok login/)).not.toBeInTheDocument();
    expect(screen.queryByText(/xai_oauth/)).not.toBeInTheDocument();
  });

  it("does not send xAI expiry to a CLI command via SubscriptionQuotaView", () => {
    render(
      <SubscriptionQuotaView
        quota={expiredQuota}
        loading={false}
        refetch={() => {}}
        appIdForExpiredHint="xai_oauth"
      />,
    );

    expect(screen.getByText(/认证中心/)).toBeInTheDocument();
    expect(
      screen.queryByText(/请运行 xai_oauth 命令刷新登录/),
    ).not.toBeInTheDocument();
  });
});

describe("xAI OAuth expiry locale copy", () => {
  it("mentions Auth Center in every locale and never grok login", () => {
    expect(zh.subscription.xaiOauthExpiredHint).toMatch(/认证中心/);
    expect(en.subscription.xaiOauthExpiredHint).toMatch(
      /Auth Center|Authentication Center/,
    );
    expect(ja.subscription.xaiOauthExpiredHint).toMatch(/認証センター/);
    expect(zhTW.subscription.xaiOauthExpiredHint).toMatch(/驗證中心|認證中心/);

    for (const locale of [zh, en, ja, zhTW]) {
      expect(locale.subscription.xaiOauthExpiredHint).not.toContain(
        "grok login",
      );
      expect(locale.subscription.xaiOauthExpiredHint).not.toContain(
        "xai_oauth",
      );
    }
  });
});
