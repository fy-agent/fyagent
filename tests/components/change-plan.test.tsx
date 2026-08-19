import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChangePlanFlow } from "@/components/change-plan/ChangePlanFlow";

const api = vi.hoisted(() => ({
  createCodexProviderSwitchPlan: vi.fn(),
  apply: vi.fn(),
  getJob: vi.fn(),
  listRecoverableJobs: vi.fn(),
}));

vi.mock("@/lib/api/change-plan", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/change-plan")>(
    "@/lib/api/change-plan",
  );
  return { ...actual, changePlanApi: api };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

const plan = {
  planId: "plan-1",
  operation: "codex_provider_switch" as const,
  targetProviderId: "provider-2",
  targetProviderName: "Provider 2",
  planDigest: "digest",
  baselineDigest: "baseline",
  createdAt: 100,
  expiresAt: Math.floor(Date.now() / 1000) + 900,
  status: "ready" as const,
  currentProviderCode: "current_configured",
  targetProviderCode: "existing_provider",
  restartExpectation: "recommended" as const,
  risks: [{ code: "local_configuration_write", severity: "notice" }],
  evidenceNote: "usage_not_observed",
};

const baseJob = {
  jobId: "job-1",
  planId: "plan-1",
  targetProviderId: "provider-2",
  revision: 3,
  eventSeq: 3,
  status: "succeeded" as const,
  resultCode: "applied" as const,
  steps: [
    { kind: "precheck" as const, status: "succeeded" as const, code: "ok" },
    { kind: "apply" as const, status: "succeeded" as const, code: "ok" },
    { kind: "readback" as const, status: "succeeded" as const, code: "ok" },
    { kind: "reconcile" as const, status: "pending" as const, code: "pending" },
  ],
  resources: [
    {
      kind: "provider_db_current" as const,
      status: "matched" as const,
      code: "ok",
    },
    { kind: "device_current" as const, status: "matched" as const, code: "ok" },
    {
      kind: "target_definition" as const,
      status: "matched" as const,
      code: "ok",
    },
    {
      kind: "codex_live_projection" as const,
      status: "matched" as const,
      code: "ok",
    },
  ],
  restartRequirement: "not_required" as const,
  usageEvidence: "not_observed" as const,
  recoveryState: "not_needed" as const,
  diagnosticCode: "ok",
  liveConfigChanged: false,
  createdAt: 100,
  updatedAt: 101,
};

function renderFlow(onOpenChange = vi.fn()) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <ChangePlanFlow
        open
        targetProviderId="provider-2"
        onOpenChange={onOpenChange}
      />
    </QueryClientProvider>,
  );
  return onOpenChange;
}

beforeEach(() => {
  vi.clearAllMocks();
  api.createCodexProviderSwitchPlan.mockResolvedValue(plan);
  api.getJob.mockResolvedValue(baseJob);
  api.listRecoverableJobs.mockResolvedValue([]);
});

describe("change-plan shared dialog", () => {
  it("renders the immutable preview and keeps the evidence boundary visible", async () => {
    renderFlow();
    expect(await screen.findByText("Provider 2")).toBeInTheDocument();
    expect(
      screen.getByText("changePlan.evidenceNotObserved"),
    ).toBeInTheDocument();
    const confirm = screen.getByRole("button", { name: "changePlan.confirm" });
    await waitFor(() => expect(confirm).toHaveFocus());
  });

  it("renders terminal readback and recovery-required states exhaustively", async () => {
    const recoveryJob = {
      ...baseJob,
      status: "failed" as const,
      resultCode: "post_write_mismatch" as const,
      recoveryState: "recovery_required" as const,
      resources: baseJob.resources.map((resource, index) =>
        index === 3 ? { ...resource, status: "mismatched" as const } : resource,
      ),
    };
    api.apply.mockResolvedValue({ kind: "admitted", job: recoveryJob });
    api.getJob.mockResolvedValue(recoveryJob);
    renderFlow();
    fireEvent.click(
      await screen.findByRole("button", { name: "changePlan.confirm" }),
    );
    expect(
      await screen.findByText("changePlan.recoveryTitle"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("changePlan.resourceStatus.mismatched"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("changePlan.evidenceNotObserved"),
    ).toBeInTheDocument();
  });

  it("offers a fresh preview after a stale admission without direct fallback", async () => {
    api.apply.mockResolvedValue({ kind: "rejected", errorCode: "stale" });
    renderFlow();
    fireEvent.click(
      await screen.findByRole("button", { name: "changePlan.confirm" }),
    );
    expect(
      await screen.findByText("changePlan.staleDescription"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "changePlan.replan" }),
    ).toBeInTheDocument();
  });

  it("keeps the dialog open while apply is running", async () => {
    api.apply.mockImplementation(() => new Promise(() => undefined));
    const onOpenChange = renderFlow();
    fireEvent.click(
      await screen.findByRole("button", { name: "changePlan.confirm" }),
    );
    expect(
      await screen.findByText("changePlan.runningTitle"),
    ).toBeInTheDocument();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onOpenChange).not.toHaveBeenCalled();
  });
});
