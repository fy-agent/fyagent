import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  createHealthPort,
  parseAgentHealthSnapshot,
} from "@/shared/platform/tauri/feature-ports/health";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { healthSnapshotFixture } from "../fixtures/health";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);
beforeEach(() => {
  invokeMock.mockReset();
});

describe("health native boundary", () => {
  it("uses the exact read-only command and payload and parses before returning", async () => {
    const { createTauriFeaturePorts } = await import(
      "@/shared/platform/tauri/features"
    );
    const { health } = createTauriFeaturePorts();
    expect(invokeMock).not.toHaveBeenCalled();
    const snapshot = healthSnapshotFixture();
    invokeMock.mockResolvedValue(snapshot);
    await expect(health.get("codex")).resolves.toEqual(snapshot);
    expect(invokeMock.mock.calls).toEqual([
      ["get_agent_health", { agentId: "codex" }],
    ]);
    invokeMock.mockResolvedValue({ ...snapshot, contractVersion: 2 });
    await expect(health.get("codex")).rejects.toThrow("无法确认运行状态");
  });

  it("rejects missing, duplicate, mismatched, excess and malformed contracts", () => {
    const valid = healthSnapshotFixture();
    for (const value of [
      null,
      {},
      { ...valid, contractVersion: 2 },
      { ...valid, agentId: "opencode" },
      { ...valid, token: "private" },
      { ...valid, checks: valid.checks.slice(1) },
      { ...valid, checks: valid.checks.map(() => valid.checks[0]) },
      { ...valid, checkedAt: "2026-02-30T00:00:00Z" },
      { ...valid, checkedAt: "2026-09-09T00:00:00+08:00" },
    ]) {
      expect(() => parseAgentHealthSnapshot(value, "codex")).toThrow(
        "无法确认运行状态",
      );
    }
    for (const patch of [
      { state: "healthy" },
      { reasonCode: "arbitrary error" },
      { severity: "fatal" },
      { action: "https://example.test" },
      { raw: "secret" },
      { checkedAt: "tomorrow" },
      { evidenceAt: "2099-01-01T00:00:00Z" },
      { state: "not_supported", severity: "error" },
    ]) {
      expect(() =>
        parseAgentHealthSnapshot(
          {
            ...valid,
            checks: [
              { ...valid.checks[0], ...patch },
              ...valid.checks.slice(1),
            ],
          },
          "codex",
        ),
      ).toThrow("无法确认运行状态");
    }
  });

  it("limits value disclosure to safe installation and model labels", () => {
    for (const value of [
      "https://example.test/key",
      "/Users/name/private",
      "C:\\Users\\name",
      "sk-secret",
      "xai-credential",
      "Bearer hidden",
      "eyJabcdefgh",
      "foo\nbar",
      "a".repeat(81),
    ]) {
      expect(() =>
        parseAgentHealthSnapshot(
          healthSnapshotFixture("codex", { model: { value } }),
          "codex",
        ),
      ).toThrow("无法确认运行状态");
    }
    expect(() =>
      parseAgentHealthSnapshot(
        healthSnapshotFixture("codex", { auth: { value: "account-name" } }),
        "codex",
      ),
    ).toThrow();
    expect(
      parseAgentHealthSnapshot(
        healthSnapshotFixture("codex", {
          model: { value: "provider/model-1.2" },
        }),
        "codex",
      ).checks,
    ).toHaveLength(12);
  });

  it("validates the UTC calendar and time while retaining chrono nanosecond precision", () => {
    for (const value of [
      "2024-02-29T23:59:59Z",
      "2024-02-29T23:59:59.123456789+00:00",
    ]) {
      expect(
        parseAgentHealthSnapshot(
          healthSnapshotFixture("codex", {}, value),
          "codex",
        ).checkedAt,
      ).toBe(value);
    }
    for (const value of [
      "2025-02-29T00:00:00Z",
      "2024-04-31T00:00:00Z",
      "2024-01-01T24:00:00Z",
      "2024-01-01T23:60:00Z",
      "2024-01-01T23:59:60Z",
      "2024-01-01T00:00Z",
      "2024-01-01T00:00:00-00:00",
    ]) {
      expect(() =>
        parseAgentHealthSnapshot(
          healthSnapshotFixture("codex", {}, value),
          "codex",
        ),
      ).toThrow("无法确认运行状态");
    }
  });

  it("does not expose raw rejection details and browser runtime never invents observations", async () => {
    invokeMock.mockRejectedValue(new Error("sk-private https://private.test"));
    await expect(createHealthPort().get("codex")).rejects.toThrow(
      "无法确认运行状态，请重新检查",
    );
    await expect(
      createBrowserFeaturePorts().health.get("codex"),
    ).rejects.toThrow("仅在 FyAgent 桌面应用");
  });
});
