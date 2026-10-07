import { SessionStages } from "@/pages/sessions/components/SessionStages";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StatusBanners } from "@/pages/sessions/components/StatusBanners";
import {
  classifyRestoreResults,
  type RestoreAttempt,
} from "@/shared/features/session-migration";
import { restoreAttemptSample } from "./samples";
import {
  restoreAttemptFeedback,
  canRetryRestoreRequest,
} from "@/pages/sessions/failure-feedback";

function attempt(overrides: Record<string, unknown> = {}): RestoreAttempt {
  return restoreAttemptSample(overrides) as unknown as RestoreAttempt;
}

describe("session migration verification presentation", () => {
  it("uses persisted stage for write truth while leaving an unrecorded failure phase explicit", () => {
    const error = {
      code: "nativeProtocolFailed",
      detail: { reason: "timeout SECRET" },
    };
    const failed = attempt({ stage: "failed", lastError: error });
    expect(restoreAttemptFeedback(failed).phase).toContain("失败阶段未记录");
    expect(restoreAttemptFeedback(failed).writeSummary).toContain(
      "没有写入副作用",
    );
    expect(
      restoreAttemptFeedback(
        attempt({ stage: "needsReconciliation", lastError: error }),
      ).writeSummary,
    ).toContain("尚未确认");
    expect(
      restoreAttemptFeedback(
        attempt({ stage: "nativeWritten", lastError: error }),
      ).writeSummary,
    ).toContain("已写入目标会话");
    expect(restoreAttemptFeedback(failed).message).not.toContain("SECRET");
  });

  it("requires the complete selected request for a safe retry, excluding foreign, missing and mixed receipts", () => {
    const first = attempt({
      stage: "failed",
      requestId: "same",
      snapshotId: "first",
    });
    const second = attempt({
      stage: "failed",
      requestId: "same",
      snapshotId: "second",
    });
    const request = {
      requestId: "same",
      snapshotIds: ["first", "second"],
      targetProviderId: "codex",
      requestKind: "defaultImport" as const,
      packagePath: "/tmp/package.json",
      targetWorkspace: "/tmp/original-workspace",
    };
    expect(canRetryRestoreRequest([first, second], request)).toBe(true);
    for (const receipts of [
      [first],
      [first, { ...second, requestId: "other" }],
      [first, { ...second, requestKind: "saveAsNewCopy" as const }],
      [first, { ...second, stage: "nativeWritten" as const }],
      [first, { ...second, stage: "needsReconciliation" as const }],
    ]) {
      expect(canRetryRestoreRequest(receipts, request)).toBe(false);
    }
  });
  it("keeps a user claim orthogonal to the native-written system stage", () => {
    render(
      <StatusBanners
        stage="nativeWritten"
        activeAttempt={attempt({
          stage: "nativeWritten",
          userAttestation: {
            attestedAt: 1_795_478_402_000,
            claimedStage: "nextTurnReplyVerified",
            note: "用户自报已续聊",
          },
        })}
      />,
    );

    expect(
      screen.getByText("已写入目标存储 · 待读回验证", { exact: true }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/用户自报标记：已手动确认续聊/u),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("真实模型续聊回复验证通过", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("does not hide a disabled capability warning after user attestation", () => {
    render(
      <StatusBanners
        stage="nativeWritten"
        isCapabilityVerified={false}
        activeAttempt={attempt({
          userAttestation: {
            attestedAt: 1_795_478_402_000,
            claimedStage: "nextTurnReplyVerified",
          },
        })}
      />,
    );

    expect(
      screen.getByText("当前版本的恢复能力尚未验证", { exact: true }),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("真实模型续聊回复验证通过", { exact: true }),
    ).not.toBeInTheDocument();
  });

  it("presents source extraction separately from the target writer and system receipt", () => {
    render(
      <SessionStages
        sourceRead="available"
        extraction="unavailable"
        probe={{
          providerId: "codex",
          installed: true,
          detectedVersion: "0.154.0",
          extractionSupported: false,
          writeSupported: true,
          reasonCode: "extractionRuleUnavailable",
        }}
        attempt={attempt({
          stage: "nativeWritten",
          userAttestation: {
            attestedAt: 1_795_478_402_000,
            claimedStage: "nextTurnReplyVerified",
          },
        })}
      />,
    );
    expect(screen.getByText("原文已读取", { exact: true })).toBeVisible();
    expect(
      screen.getByText("提取不可用；可用原文仍可查看", { exact: true }),
    ).toBeVisible();
    expect(screen.getByText(/本机版本支持写入恢复/u)).toBeVisible();
    expect(
      screen.getByText("已写入目标存储 · 待读回验证", { exact: true }),
    ).toBeVisible();
    expect(
      screen.getByText("会话包已核验并建立恢复回执", { exact: true }),
    ).toBeVisible();
    expect(
      screen.getByText("系统已确认写入目标存储", { exact: true }),
    ).toBeVisible();
    expect(
      screen.getByText("目标读回尚未核验通过", { exact: true }),
    ).toBeVisible();
    expect(
      screen.getByText("软件实际打开、重启和真实续聊闭环尚未核验", {
        exact: true,
      }),
    ).toBeVisible();
    expect(
      screen.queryByText("真实模型续聊回复验证通过", { exact: true }),
    ).toBeNull();
  });

  it("keeps user attestation visible after readback without a second self-report action", () => {
    render(
      <StatusBanners
        stage="nativeReadbackVerified"
        activeAttempt={attempt({
          stage: "nativeReadbackVerified",
          userAttestation: {
            attestedAt: 1_795_478_402_000,
            claimedStage: "nextTurnReplyVerified",
          },
        })}
      />,
    );
    expect(screen.getByText("目标历史核验通过", { exact: true })).toBeVisible();
    expect(screen.getByText(/用户自报标记：已手动确认续聊/u)).toBeVisible();
    expect(
      screen.queryByText("真实模型续聊回复验证通过", { exact: true }),
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: "标记：我已手动续聊" }),
    ).toBeNull();
  });

  it.each(["nativeReadbackVerified", "nextTurnRequestVerified"] as const)(
    "does not promote %s to a real model reply from user attestation",
    (stage) => {
      render(
        <SessionStages
          attempt={attempt({
            stage,
            userAttestation: {
              attestedAt: 1_795_478_402_000,
              claimedStage: "nextTurnReplyVerified",
            },
          })}
        />,
      );
      expect(
        screen.getByText("系统已核验目标本地历史", { exact: true }),
      ).toBeVisible();
      expect(
        screen.queryByText("系统已验证真实模型回复", { exact: true }),
      ).toBeNull();
      expect(
        screen.queryByText("真实模型续聊回复验证通过", { exact: true }),
      ).toBeNull();
    },
  );

  it("keeps user-provided notes out of the default attestation summary", () => {
    render(
      <StatusBanners
        stage="nativeReadbackVerified"
        activeAttempt={attempt({
          stage: "nativeReadbackVerified",
          userAttestation: {
            attestedAt: 1_795_478_402_000,
            claimedStage: "nextTurnReplyVerified",
            note: "private user note",
          },
        })}
      />,
    );
    expect(screen.getByText(/用户自报标记：已手动确认续聊/u)).toBeVisible();
    expect(
      screen.getByText("private user note", { exact: true }),
    ).not.toBeVisible();
  });

  it("presents unresolved side effects as reconciliation, not safe failure", () => {
    render(
      <StatusBanners
        stage="needsReconciliation"
        activeAttempt={attempt({ stage: "needsReconciliation" })}
      />,
    );

    expect(
      screen.getByText("写入结果尚未确认", { exact: true }),
    ).toBeInTheDocument();
    expect(screen.getByText(/禁止盲目重试/u)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /重试/u }),
    ).not.toBeInTheDocument();
  });

  it("distinguishes a genuinely incomplete user turn from indeterminate assistant output", () => {
    const { rerender } = render(
      <StatusBanners hasIncompleteTurn isIndeterminate={false} />,
    );
    expect(
      screen.getByText("包含未完成轮次 · 仅保留用户提示词原文", {
        exact: true,
      }),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/无法可靠确定最终答复原文/u),
    ).not.toBeInTheDocument();

    rerender(<StatusBanners hasIncompleteTurn isIndeterminate />);
    expect(screen.getByText(/无法可靠确定最终答复原文/u)).toBeInTheDocument();
  });
  it("never presents native readback as a real model reply", () => {
    render(<StatusBanners stage="nativeReadbackVerified" />);
    expect(
      screen.getByText("目标历史核验通过", { exact: true }),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("真实模型续聊回复验证通过", { exact: true }),
    ).not.toBeInTheDocument();
  });
  it("keeps written-only and mixed batches pending until every history is verified", () => {
    for (const stages of [
      ["nativeWritten"],
      ["nativeReadbackVerified", "nativeWritten"],
    ]) {
      const result = classifyRestoreResults(
        stages.map((stage) => attempt({ stage })),
      );
      expect(result.isAllCleanSuccess).toBe(false);
      expect(result.bannerTone).toBe("warning");
      expect(result.summaryTitle).toBe("已写入，待读回验证");
    }
    expect(
      classifyRestoreResults([attempt({ stage: "nativeReadbackVerified" })])
        .isAllCleanSuccess,
    ).toBe(true);
  });
});
