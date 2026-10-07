import { describe, expect, it } from "vitest";
import {
  connectionPresentation,
  summarizeAuthOverview,
  summarizeConnections,
} from "@/pages/auth/summary";
import {
  parseManagedAuthOverview,
  type ManagedAuthConnectionSummary,
  type ManagedAuthOverview,
} from "@/shared/features/managed-auth";
import { managedAuthOverviewFixture } from "../../fixtures/managedAuth";

function parsedFixtureOverview(overview: ManagedAuthOverview) {
  overview.accounts = overview.accounts.map((account) => ({
    ...account,
    connectedConsumerCount: new Set(
      overview.connections
        .filter((connection) => connection.accountId === account.accountId)
        .map((connection) => connection.consumer),
    ).size,
  }));
  return parseManagedAuthOverview(overview);
}

function fixtureSummary(connections: ManagedAuthConnectionSummary[]) {
  const overview = managedAuthOverviewFixture();
  overview.connections = connections.map((connection, index) => ({
    ...connection,
    connectionId: `mc1:${(index + 1).toString(16).padStart(32, "0")}`,
  }));
  return summarizeConnections(parsedFixtureOverview(overview).connections);
}
describe("managed auth summaries", () => {
  it("keeps empty accounts and disconnected positions distinct", () => {
    const overview = managedAuthOverviewFixture();
    overview.accounts = [];
    overview.connections = overview.connections.map((item) => ({
      ...item,
      accountId: null,
      authStatus: "disconnected",
      requestMode: "none",
      requestProviderLabel: null,
    }));
    const summary = summarizeAuthOverview(parsedFixtureOverview(overview));
    expect(summary.label).toBe("尚未保存受管账号 · 未连接");
    expect(summary.connections.countsLabel).toBe("4 个连接位置 · 0 个已连接");
    expect(summary.connections.tone).toBe("neutral");
  });

  it("uses the same partial status with explicit remaining states", () => {
    const [connected, disconnected] = managedAuthOverviewFixture().connections;
    const result = fixtureSummary([
      connected,
      { ...disconnected, accountId: null, authStatus: "disconnected" },
    ]);
    expect(result).toMatchObject({
      total: 2,
      connected: 1,
      label: "部分连接 1/2",
      summary: "部分连接 1/2 · 未连接",
      tone: "neutral",
    });
  });

  it("never counts saved credentials or pending pickup as connected", () => {
    const [connection] = managedAuthOverviewFixture().connections;
    const saved = {
      ...connection,
      reasonCodes: ["native_projection_unavailable" as const],
    };
    expect(fixtureSummary([saved])).toMatchObject({
      connected: 0,
      label: "账号已保存",
      summary: "账号已保存，尚未确认软件连接",
      tone: "warning",
    });
    expect(
      fixtureSummary([{ ...connection, authStatus: "disconnected" }]),
    ).toMatchObject({ connected: 0, label: "账号已保存" });
    expect(
      fixtureSummary([
        { ...connection, authStatus: "pending_restart", pendingRestart: true },
      ]),
    ).toMatchObject({ connected: 0, label: "等待重启" });
  });

  it.each([
    "checking",
    "requires_reauth",
    "pending_restart",
    "unavailable",
  ] as const)("retains %s alongside a connected position", (authStatus) => {
    const [connection] = managedAuthOverviewFixture().connections;
    const pending = {
      ...connection,
      authStatus,
      pendingRestart: authStatus === "pending_restart",
    };
    const result = fixtureSummary([connection, pending]);
    expect(result.label).toBe("部分连接 1/2");
    expect(result.summary).toContain(connectionPresentation(pending).label);
    expect(result.connected).toBe(1);
    expect(result.tone).toBe(authStatus === "checking" ? "neutral" : "warning");
  });

  it("does not infer managed binding from native login or request source", () => {
    const [connection] = managedAuthOverviewFixture().connections;
    const nativeOnly = {
      ...connection,
      accountId: null,
      authStatus: "disconnected" as const,
      officialSessionPreserved: true,
      requestMode: "third_party_api" as const,
    };
    expect(fixtureSummary([nativeOnly])).toMatchObject({
      connected: 0,
      label: "未连接",
    });
    expect(nativeOnly.officialSessionPreserved).toBe(true);
    expect(nativeOnly.requestMode).toBe("third_party_api");
    expect(
      fixtureSummary([
        {
          ...nativeOnly,
          authStatus: "unavailable",
          requestMode: "unknown",
          requestProviderLabel: null,
        },
      ]),
    ).toMatchObject({ connected: 0, label: "状态不可用" });
  });
  it("does not turn saved ready accounts into connected positions", () => {
    const overview = managedAuthOverviewFixture();
    overview.connections = overview.connections.map((item) => ({
      ...item,
      accountId: null,
      authStatus: "disconnected",
    }));
    expect(
      summarizeAuthOverview(parsedFixtureOverview(overview)),
    ).toMatchObject({
      label: "受管账号状态正常 · 未连接",
      connections: { connected: 0, total: 4, label: "未连接" },
    });
  });

  it("keeps all-connected and no-position groups explicit", () => {
    expect(
      fixtureSummary(managedAuthOverviewFixture().connections),
    ).toMatchObject({
      connected: 4,
      total: 4,
      label: "已连接 4/4",
      tone: "accent",
    });
    expect(fixtureSummary([])).toMatchObject({
      connected: 0,
      total: 0,
      label: "暂时没有可管理的连接",
      tone: "neutral",
    });
  });

  it("retains unreadable request source without changing connection authority", () => {
    const overview = managedAuthOverviewFixture();
    overview.connections[0] = {
      ...overview.connections[0],
      requestMode: "unknown",
      requestProviderLabel: null,
    };
    expect(
      summarizeAuthOverview(parsedFixtureOverview(overview)),
    ).toMatchObject({
      label: "受管账号状态正常 · 已连接 4/4 · 1 个位置的请求来源暂时无法确认",
      connections: { connected: 4 },
    });
  });
});
