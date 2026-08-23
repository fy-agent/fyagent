import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import fixtureJson from "../../../fixtures/changePlanDtoContract.v1.json";
import { ChangePlanSwitch } from "@/v2/pages/models/change-plan/ChangePlanSwitch";
import type {
  ChangeJobSnapshot,
  ChangeJobUpdatedEvent,
  ChangePlan,
} from "@/v2/shared/features/change-plan";
import type { ChangePlanPort } from "@/v2/shared/features/ports";
import { TooltipProvider } from "@/v2/shared/ui/primitives";

const planFixture = (): ChangePlan =>
  structuredClone(fixtureJson.plan) as unknown as ChangePlan;
const terminalFixture = (): ChangeJobSnapshot =>
  structuredClone(fixtureJson.applyOutcome.job) as unknown as ChangeJobSnapshot;

function runningFixture(eventSeq = 4): ChangeJobSnapshot {
  const job = terminalFixture();
  job.eventSeq = eventSeq;
  job.revision = eventSeq;
  job.status = "running";
  job.resultCode = "running";
  job.restartRequirement = "unknown";
  job.diagnosticCode = "managed_write_started";
  job.steps = job.steps.map((step) => ({
    ...step,
    status:
      step.kind === "precheck" || step.kind === "snapshot"
        ? "succeeded"
        : step.kind === "managed_write"
          ? "running"
          : "not_started",
  }));
  job.resources = job.resources.map((resource) => ({
    ...resource,
    status: "pending",
  }));
  job.partialResult = {
    succeededSteps: ["precheck", "snapshot"],
    compensatedSteps: [],
    unverifiedSteps: ["managed_write"],
    remainingEffects: [],
    manualActions: [],
  };
  return job;
}

function cancelledFixture(): ChangeJobSnapshot {
  const job = runningFixture(5);
  job.status = "cancelled";
  job.resultCode = "cancelled_before_write";
  job.recoveryState = "not_needed";
  job.partialResult = undefined;
  job.steps = job.steps.map((step) => ({
    ...step,
    status:
      step.kind === "precheck" ||
      step.kind === "snapshot" ||
      step.kind === "finalize"
        ? "succeeded"
        : "skipped",
  }));
  return job;
}

function createPort(overrides: Partial<ChangePlanPort> = {}): {
  port: ChangePlanPort;
  emit: (event: ChangeJobUpdatedEvent) => void;
} {
  let listener: ((event: ChangeJobUpdatedEvent) => void) | undefined;
  const port: ChangePlanPort = {
    createCodexProviderSwitchPlan: vi.fn(async () => planFixture()),
    apply: vi.fn<ChangePlanPort["apply"]>(async () => ({
      kind: "admitted",
      job: terminalFixture(),
    })),
    getJob: vi.fn(async () => terminalFixture()),
    listRecoverableJobs: vi.fn(async () => []),
    cancelJob: vi.fn<ChangePlanPort["cancelJob"]>(async (jobId) => ({
      accepted: true,
      code: "accepted",
      jobId,
    })),
    subscribeJobUpdates: vi.fn(async (onEvent) => {
      listener = onEvent;
      return vi.fn();
    }),
    ...overrides,
  };
  return {
    port,
    emit: (event) => listener?.(event),
  };
}

function renderSwitch(port: ChangePlanPort, onTerminal = vi.fn()) {
  return render(
    <TooltipProvider delayDuration={0} skipDelayDuration={0}>
      <ChangePlanSwitch
        active
        currentProviderId="provider-current"
        providers={{
          "provider-current": { id: "provider-current", name: "Current" },
          "provider-target": { id: "provider-target", name: "Target Provider" },
        }}
        port={port}
        onTerminal={onTerminal}
      />
    </TooltipProvider>,
  );
}

describe("V2 Change Plan switch", () => {
  it("previews one real plan, confirms once, and renders event/readback truth", async () => {
    const user = userEvent.setup();
    const admitted = runningFixture(1);
    admitted.steps = admitted.steps.map((step) => ({
      ...step,
      status: step.kind === "precheck" ? "running" : "not_started",
    }));
    const running = runningFixture();
    let authoritative = running;
    const onTerminal = vi.fn();
    const { port, emit } = createPort({
      apply: vi.fn<ChangePlanPort["apply"]>(async () => ({
        kind: "admitted",
        job: admitted,
      })),
      getJob: vi.fn(async () => authoritative),
    });
    renderSwitch(port, onTerminal);
    await waitFor(() =>
      expect(port.subscribeJobUpdates).toHaveBeenCalledTimes(1),
    );

    expect(screen.getByRole("button", { name: "正在使用" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "预览切换" }));

    const dialog = await screen.findByRole("dialog", {
      name: "确认 Codex 配置切换",
    });
    expect(dialog).toHaveTextContent("语义变化");
    expect(dialog).toHaveTextContent("风险与重启");
    expect(dialog).toHaveTextContent("前置条件与范围");
    expect(dialog).toHaveTextContent("恢复方式");
    expect(dialog).toHaveTextContent("Target Provider");
    expect(dialog).not.toHaveTextContent("plan-digest");
    expect(dialog).not.toHaveTextContent("baseline-digest");
    expect(port.apply).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "确认并应用一次" }));
    expect(port.apply).toHaveBeenCalledTimes(1);
    expect(port.apply).toHaveBeenCalledWith("plan-contract", "plan-digest");
    expect(
      vi.mocked(port.subscribeJobUpdates).mock.invocationCallOrder[0],
    ).toBeLessThan(vi.mocked(port.apply).mock.invocationCallOrder[0]);
    expect(await screen.findByTestId("change-job-workspace")).toBeVisible();

    await act(async () => {
      emit({ jobId: running.jobId, eventSeq: running.eventSeq });
    });
    expect(await screen.findByText("写入受管配置")).toBeVisible();
    expect(screen.getByText("进行中")).toBeVisible();

    authoritative = terminalFixture();
    await act(async () => {
      emit({
        jobId: authoritative.jobId,
        eventSeq: authoritative.eventSeq,
      });
    });
    expect(
      await screen.findByText(
        "配置已应用，可直接开始使用；建议重启或新建会话。",
      ),
    ).toBeVisible();
    expect(
      screen.getByText("真实使用证据：配置已应用，尚无真实使用证据"),
    ).toBeVisible();
    await waitFor(() => expect(onTerminal).toHaveBeenCalledTimes(1));
  });

  it("requests cancellation only at the pre-write safe point", async () => {
    const user = userEvent.setup();
    const pending = runningFixture(3);
    pending.steps = pending.steps.map((step) => ({
      ...step,
      status:
        step.kind === "precheck" || step.kind === "snapshot"
          ? "succeeded"
          : "not_started",
    }));
    const cancelled = cancelledFixture();
    const { port } = createPort({
      listRecoverableJobs: vi.fn(async () => [pending]),
      getJob: vi.fn(async () => cancelled),
    });
    renderSwitch(port);

    const cancel = await screen.findByRole("button", { name: "写入前取消" });
    await user.click(cancel);
    expect(port.cancelJob).toHaveBeenCalledWith(pending.jobId);
    expect(
      await screen.findByText("已在首笔受管写入前取消，目标配置未写入。"),
    ).toBeVisible();
    expect(port.apply).not.toHaveBeenCalled();
  });

  it("never renders backend codes, digests, secret canaries, or stale event regressions", async () => {
    const secret = "SECRET-CANARY-NOT-FOR-DOM-824";
    const newer = runningFixture(9);
    newer.diagnosticCode = secret;
    newer.steps[2].code = secret;
    newer.resources[0].code = secret;
    newer.partialResult = {
      succeededSteps: ["precheck", "snapshot"],
      compensatedSteps: [],
      unverifiedSteps: ["managed_write"],
      remainingEffects: [secret],
      manualActions: [secret],
    };
    const getJob = vi.fn(async () => newer);
    const { port, emit } = createPort({
      listRecoverableJobs: vi.fn(async () => [newer]),
      getJob,
    });
    const view = renderSwitch(port);
    expect(await screen.findByTestId("change-job-workspace")).toBeVisible();
    expect(view.container).not.toHaveTextContent(secret);
    expect(view.container).not.toHaveTextContent("plan-digest");

    const callsBefore = getJob.mock.calls.length;
    await act(async () => {
      emit({ jobId: newer.jobId, eventSeq: 8 });
      emit({ jobId: "unrelated-job", eventSeq: 99 });
    });
    expect(getJob).toHaveBeenCalledTimes(callsBefore);
    expect(view.container).not.toHaveTextContent(secret);
  });
});
