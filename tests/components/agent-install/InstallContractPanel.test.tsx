import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { InstallContractPanel } from "@/components/agent-install/InstallContractPanel";
import type { InstallContract } from "@/types/agentInstall";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

function contract(overrides: Partial<InstallContract> = {}): InstallContract {
  return {
    schema: "fyagent-agent-install-contract-v1",
    agentId: "codexCli",
    catalog: {
      agentId: "codexCli",
      sourceState: "ok",
      officialLandingUrl: "https://github.com/openai/codex",
      legalEntity: "OpenAI",
      licenseUrl: "https://github.com/openai/codex/blob/main/LICENSE",
      licenseScope: "public_open_source",
      packageSourceKind: "package_manager",
      cacheAllowed: true,
      redistributionAllowed: true,
      installMode: "package_manager",
      evidenceUrl: null,
      checkedAt: "t0",
      writtenPermissionNeeded: false,
    },
    package: {
      integrityState: "warn",
      hash: { state: "unknown", value: null },
      signature: { state: "unknown", value: null },
      revocation: { state: "unknown", value: null },
      verificationSource: ["package_manager_metadata"],
      integritySummary: "package_manager_metadata",
      checkedAt: "t0",
    },
    environment: {
      preflightState: "ok",
      checks: [],
      checkedAt: "t0",
    },
    plan: {
      planSnapshotId: "snap-1",
      planHash: "abc",
      snapshotStale: false,
      driftReasons: [],
      refreshedAt: "t0",
    },
    updatedAt: "t0",
    installAllowed: true,
    guideAllowed: true,
    ...overrides,
  };
}

describe("InstallContractPanel", () => {
  it("renders four independent cards", () => {
    render(
      <InstallContractPanel
        contract={contract()}
        onOpenGuide={vi.fn()}
        onRecheck={vi.fn()}
        onRegenerate={vi.fn()}
        onInstall={vi.fn()}
      />,
    );
    expect(screen.getByText("agentInstall.source")).toBeTruthy();
    expect(screen.getByText("agentInstall.integrity")).toBeTruthy();
    expect(screen.getByText("agentInstall.preflight")).toBeTruthy();
    expect(screen.getByText("agentInstall.plan")).toBeTruthy();
  });

  it("unknown disables install", () => {
    render(
      <InstallContractPanel
        contract={contract({
          installAllowed: false,
          environment: {
            preflightState: "unknown",
            checks: [],
            checkedAt: "t0",
          },
        })}
        onOpenGuide={vi.fn()}
        onRecheck={vi.fn()}
        onRegenerate={vi.fn()}
        onInstall={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "agentInstall.install" }),
    ).toHaveProperty("disabled", true);
  });

  it("warn enables install with warning", () => {
    render(
      <InstallContractPanel
        contract={contract({ installAllowed: true })}
        onOpenGuide={vi.fn()}
        onRecheck={vi.fn()}
        onRegenerate={vi.fn()}
        onInstall={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "agentInstall.install" }),
    ).toHaveProperty("disabled", false);
    expect(screen.getByText("agentInstall.warnContinue")).toBeTruthy();
  });

  it("stale plan requires reconfirm", () => {
    render(
      <InstallContractPanel
        contract={contract({
          installAllowed: true,
          plan: {
            planSnapshotId: "snap-1",
            planHash: "abc",
            snapshotStale: true,
            driftReasons: ["package_hash"],
            refreshedAt: "t0",
          },
        })}
        onOpenGuide={vi.fn()}
        onRecheck={vi.fn()}
        onRegenerate={vi.fn()}
        onInstall={vi.fn()}
      />,
    );
    expect(screen.getAllByText("agentInstall.regeneratePlan").length).toBeGreaterThan(0);
  });
});
