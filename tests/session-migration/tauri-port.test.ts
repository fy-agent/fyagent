import { beforeEach, describe, expect, it, vi } from "vitest";

import { createSessionMigrationPort } from "@/shared/platform/tauri/feature-ports/sessionMigration";
import {
  disabledProbeSample,
  restoreAttemptSample,
  sessionPackageSample,
} from "./samples";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("session migration Tauri port", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("sends a closed restore request without package-controlled commands", async () => {
    const request = {
      packagePath: "/tmp/synthetic.fy-session.json",
      requestId: "33333333-3333-4333-8333-333333333333",
      snapshotIds: [`fys1:${"b".repeat(64)}`],
      targetProviderId: "codex",
      targetWorkspace: "/tmp/fyagent-session-migration",
      requestKind: "saveAsNewCopy" as const,
    };
    invoke.mockResolvedValue([restoreAttemptSample()]);

    const port = createSessionMigrationPort();
    await port.restoreSessionPackage(request);
    await port.restoreSessionPackage(request);

    expect(invoke).toHaveBeenNthCalledWith(1, "restore_session_package", {
      request,
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "restore_session_package", {
      request,
    });
    expect(invoke.mock.calls[0][1]).not.toHaveProperty("command");
    expect(invoke.mock.calls[0][1]).not.toHaveProperty("argv");
  });

  it("rejects request extras and overwrite before native invocation", async () => {
    const port = createSessionMigrationPort();
    const unsafe = {
      packagePath: "/tmp/synthetic.fy-session.json",
      requestId: "33333333-3333-4333-8333-333333333333",
      snapshotIds: [],
      targetProviderId: "codex",
      targetWorkspace: "/tmp/fyagent-session-migration",
      requestKind: "overwrite",
      command: "codex resume attacker-controlled",
    };

    await expect(
      port.restoreSessionPackage(
        unsafe as Parameters<typeof port.restoreSessionPackage>[0],
      ),
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("parses unsupported capability as disabled without a user override path", async () => {
    invoke.mockResolvedValue(disabledProbeSample());
    const probe =
      await createSessionMigrationPort().probeLocalProvider("grokbuild");

    expect(invoke).toHaveBeenCalledWith("probe_local_provider", {
      providerId: "grokbuild",
    });
    expect(probe).toMatchObject({
      providerId: "grokbuild",
      writeSupported: false,
      extractionSupported: false,
      reasonCode: "providerVersionUnsupported",
    });
  });

  it("records user attestation without accepting it as a system stage", async () => {
    invoke.mockResolvedValue(
      restoreAttemptSample({
        stage: "nativeWritten",
        userAttestation: {
          attestedAt: 1_795_478_402_000,
          claimedStage: "nextTurnReplyVerified",
        },
      }),
    );

    const attempt = await createSessionMigrationPort().recordUserAttestation(
      "22222222-2222-4222-8222-222222222222",
      "nextTurnReplyVerified",
    );

    expect(invoke).toHaveBeenCalledWith("record_user_attestation", {
      attemptId: "22222222-2222-4222-8222-222222222222",
      claimedStage: "nextTurnReplyVerified",
      note: undefined,
    });
    expect(attempt.stage).toBe("nativeWritten");
    expect(attempt.userAttestation?.claimedStage).toBe("nextTurnReplyVerified");
  });

  it("rejects unknown fields returned by native package and receipt readers", async () => {
    const port = createSessionMigrationPort();
    invoke.mockResolvedValueOnce({
      package: { ...sessionPackageSample(), rawEvents: [] },
      attempts: [],
    });
    await expect(
      port.readSessionPackage("/tmp/synthetic.fy-session.json"),
    ).rejects.toThrow();

    invoke.mockResolvedValueOnce([
      { ...restoreAttemptSample(), authoritativeSuccess: true },
    ]);
    await expect(port.listRestoreAttempts()).rejects.toThrow();
  });

  it("uses the dedicated session package file pickers and parses their paths", async () => {
    const port = createSessionMigrationPort();
    invoke
      .mockResolvedValueOnce("/tmp/incoming.fy-session.json")
      .mockResolvedValueOnce("/tmp/outgoing.fy-session.json");

    await expect(port.pickPackageFile()).resolves.toBe(
      "/tmp/incoming.fy-session.json",
    );
    await expect(
      port.pickExportPath("fyagent-session-codex-2026-09-22.json"),
    ).resolves.toBe("/tmp/outgoing.fy-session.json");

    expect(invoke).toHaveBeenNthCalledWith(1, "pick_session_package_file");
    expect(invoke).toHaveBeenNthCalledWith(
      2,
      "pick_session_package_export_path",
      { defaultName: "fyagent-session-codex-2026-09-22.json" },
    );
  });
});
