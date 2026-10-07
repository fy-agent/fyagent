import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { SessionsPage } from "@/pages/sessions/Page";
import type { AttestationDialogProps } from "@/pages/sessions/components/AttestationDialog";
import type { ImportPackageDialogProps } from "@/pages/sessions/components/ImportPackageDialog";
import { FeatureProvider } from "@/shared/features/provider";
import {
  localProviderProbeSchema,
  migratableSessionSchema,
  restoreAttemptSchema,
  restoreRequestSchema,
  sessionMessageSchema,
  sessionMetaSchema,
  sessionPackageSchema,
  type RestoreRequest,
} from "@/shared/features/session-migration";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { PersistentSurface } from "@/shared/ui/PersistentSurface";
import {
  restoreAttemptSample,
  sessionPackageSample,
} from "../../../session-migration/samples";

vi.mock("@/shared/platform/runtime", () => ({
  detectRuntime: () => ({ isNative: true }),
}));

// Exercise the Page's real operation delegates and QueryClient independently
// of the dialog lifecycle already covered by failure-lifecycle.test.tsx.
const delegates = vi.hoisted(() => ({
  import: null as ImportPackageDialogProps | null,
  attestation: null as AttestationDialogProps | null,
}));
vi.mock("@/pages/sessions/components/ImportPackageDialog", () => ({
  ImportPackageDialog: (props: ImportPackageDialogProps) => {
    delegates.import = props;
    return null;
  },
}));

vi.mock("@/pages/sessions/components/AttestationDialog", () => ({
  AttestationDialog: (props: AttestationDialogProps) => {
    delegates.attestation = props;
    return null;
  },
}));

function pagePorts() {
  const ports = createBrowserFeaturePorts();
  ports.sessions.listSessions = vi.fn(async () => [
    sessionMetaSchema.parse({
      providerId: "codex",
      sessionId: "synthetic-session",
      sourcePath: "/isolated/session.jsonl",
      projectDir: "/isolated/workspace",
      title: "可见性样本",
    }),
  ]);
  ports.sessions.listRestoreAttempts = vi.fn(async () => []);
  ports.sessions.probeLocalProvider = vi.fn(async (providerId) =>
    localProviderProbeSchema.parse({
      providerId,
      installed: false,
      extractionSupported: false,
      writeSupported: false,
    }),
  );
  ports.sessions.getSessionMessages = vi.fn(async () => [
    sessionMessageSchema.parse({ role: "user", content: "读取样本" }),
  ]);
  ports.sessions.previewSessionMigration = vi.fn(async () =>
    migratableSessionSchema.parse(
      sessionPackageSchema.parse(sessionPackageSample()).sessions[0],
    ),
  );
  return ports;
}

function renderPage(ports: ReturnType<typeof pagePorts>, active = true) {
  let client!: QueryClient;
  function QueryAccess() {
    client = useQueryClient();
    return null;
  }
  function Harness({ visible }: { visible: boolean }) {
    return (
      <MemoryRouter>
        <FeatureProvider ports={ports}>
          <QueryAccess />
          <PersistentSurface active={visible}>
            <SessionsPage />
          </PersistentSurface>
        </FeatureProvider>
      </MemoryRouter>
    );
  }
  const view = render(<Harness visible={active} />);
  return {
    client: () => client,
    visibility: (visible: boolean) =>
      view.rerender(<Harness visible={visible} />),
  };
}

async function settledReads(ports: ReturnType<typeof pagePorts>) {
  await screen.findByText("可见性样本", { exact: true });
  await waitFor(() => {
    expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(1);
    expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(7);
  });
}

const request: RestoreRequest = restoreRequestSchema.parse({
  packagePath: "/isolated/package.json",
  requestId: "33333333-3333-4333-8333-333333333333",
  snapshotIds: ["fys1:" + "b".repeat(64)],
  targetProviderId: "codex",
  targetWorkspace: "/isolated/workspace",
  requestKind: "defaultImport",
});

function importDelegates() {
  if (!delegates.import) throw new Error("Page operation delegates missing");
  const props = delegates.import;
  if (!props.onReviewRestore || !props.onVerifyReadback)
    throw new Error("Page receipt readback delegates missing");
  return {
    onRestore: props.onRestore,
    onReviewRestore: props.onReviewRestore,
    onVerifyReadback: props.onVerifyReadback,
  };
}

describe("Sessions visibility query ownership", () => {
  it("defers hidden reads, stops stale observations, and resumes them with the same search draft", async () => {
    const user = userEvent.setup();
    const ports = pagePorts();
    const view = renderPage(ports, false);
    await act(async () => {
      await view.client().invalidateQueries();
    });
    expect(ports.sessions.listSessions).not.toHaveBeenCalled();
    expect(ports.sessions.listRestoreAttempts).not.toHaveBeenCalled();
    expect(ports.sessions.probeLocalProvider).not.toHaveBeenCalled();

    view.visibility(true);
    await settledReads(ports);
    const search = screen.getByRole("searchbox", { name: "搜索会话" });
    await user.type(search, "可见性");
    await user.click(screen.getByText("可见性样本", { exact: true }));
    await waitFor(() => {
      expect(ports.sessions.getSessionMessages).toHaveBeenCalledTimes(1);
      expect(ports.sessions.previewSessionMigration).toHaveBeenCalledTimes(1);
    });
    view.visibility(false);
    await act(async () => {
      await view.client().invalidateQueries();
    });
    expect(ports.sessions.listSessions).toHaveBeenCalledTimes(1);
    expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(1);
    expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(7);
    expect(ports.sessions.getSessionMessages).toHaveBeenCalledTimes(1);
    expect(ports.sessions.previewSessionMigration).toHaveBeenCalledTimes(1);

    view.visibility(true);
    await waitFor(() => {
      expect(ports.sessions.listSessions).toHaveBeenCalledTimes(2);
      expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(2);
      expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(14);
      expect(ports.sessions.getSessionMessages).toHaveBeenCalledTimes(2);
      expect(ports.sessions.previewSessionMigration).toHaveBeenCalledTimes(2);
    });
    expect(screen.getByRole("searchbox", { name: "搜索会话" })).toBe(search);
    expect(search).toHaveValue("可见性");
  });

  it.each([
    "restore",
    "restore-rejection",
    "verify",
    "review",
    "review-rejection",
  ] as const)(
    "keeps %s terminal readback fresh after the page becomes hidden",
    async (operation) => {
      const ports = pagePorts();
      const view = renderPage(ports);
      await settledReads(ports);
      const receipt = restoreAttemptSchema.parse(restoreAttemptSample());
      let release!: () => void;
      const pending = new Promise<void>((resolve) => {
        release = resolve;
      });
      const rejected = { code: "nativeProtocolFailed" };
      ports.sessions.restoreSessionPackage = vi.fn(async () => {
        await pending;
        if (operation === "restore-rejection") throw rejected;
        return [receipt];
      });
      ports.sessions.verifyNativeReadback = vi.fn(async () => {
        await pending;
        return receipt;
      });
      ports.sessions.reconcileRestoreAttempts = vi.fn(async () => {
        await pending;
        if (operation === "review-rejection") throw rejected;
        return [receipt];
      });
      let result: unknown;
      let error: unknown;
      let settled!: Promise<void>;
      await act(async () => {
        const callbacks = importDelegates();
        const work =
          operation === "verify"
            ? callbacks.onVerifyReadback(receipt.attemptId)
            : operation === "review" || operation === "review-rejection"
              ? callbacks.onReviewRestore()
              : callbacks.onRestore(request);
        settled = work.then(
          (value) => {
            result = value;
          },
          (reason: unknown) => {
            error = reason;
          },
        );
      });
      view.visibility(false);
      vi.mocked(ports.sessions.listRestoreAttempts).mockResolvedValue([
        receipt,
      ]);
      await act(async () => {
        release();
        await settled;
      });
      // The cache was populated just before the operation (inside 15s).
      // Explicit staleTime: 0 must still reread the persisted terminal receipt.
      expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(2);
      expect(view.client().getQueryData(["sessions-attempts"])).toEqual([
        receipt,
      ]);
      expect(ports.sessions.listSessions).toHaveBeenCalledTimes(1);
      expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(7);
      if (
        operation === "restore-rejection" ||
        operation === "review-rejection"
      ) {
        expect(error).toBe(rejected);
        expect(result).toBeUndefined();
      } else {
        expect(error).toBeUndefined();
        expect(result).toEqual(operation === "verify" ? receipt : [receipt]);
      }
    },
  );

  it.each(["page-verify", "attestation"] as const)(
    "rereads the persisted receipt when %s finishes while hidden",
    async (operation) => {
      const user = userEvent.setup();
      const ports = pagePorts();
      const before = restoreAttemptSchema.parse(restoreAttemptSample());
      const after = restoreAttemptSchema.parse(
        restoreAttemptSample(
          operation === "page-verify"
            ? { stage: "nativeReadbackVerified" }
            : {
                userAttestation: {
                  attestedAt: 1795478402000,
                  claimedStage: "nextTurnReplyVerified",
                  note: "用户确认",
                },
              },
        ),
      );
      vi.mocked(ports.sessions.listRestoreAttempts).mockResolvedValue([before]);
      const view = renderPage(ports);
      await settledReads(ports);
      await user.click(screen.getByText("可见性样本", { exact: true }));
      await screen.findByRole("button", { name: "系统读回核验" });
      let release!: () => void;
      const pending = new Promise<void>((resolve) => {
        release = resolve;
      });
      ports.sessions.verifyNativeReadback = vi.fn(async () => {
        await pending;
        return after;
      });
      ports.sessions.recordUserAttestation = vi.fn(async () => {
        await pending;
        return after;
      });
      let attestationWork: Promise<void> | undefined;
      if (operation === "page-verify") {
        await user.click(screen.getByRole("button", { name: "系统读回核验" }));
        expect(ports.sessions.verifyNativeReadback).toHaveBeenCalledTimes(1);
      } else {
        await user.click(
          within(
            screen.getByTestId("sessions-page").querySelector("footer")!,
          ).getByRole("button", { name: "标记：我已手动续聊" }),
        );
        const callback = delegates.attestation?.onRecord;
        if (!callback) throw new Error("Page attestation delegate missing");
        await act(async () => {
          attestationWork = callback(
            before.attemptId,
            "nextTurnReplyVerified",
            "用户确认",
          );
        });
      }
      view.visibility(false);
      vi.mocked(ports.sessions.listRestoreAttempts).mockResolvedValue([after]);
      await act(async () => {
        release();
        await attestationWork;
      });
      await waitFor(() => {
        expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(2);
        expect(view.client().getQueryData(["sessions-attempts"])).toEqual([
          after,
        ]);
      });
      expect(ports.sessions.listSessions).toHaveBeenCalledTimes(1);
      expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(7);
    },
  );

  it.each([
    "restore",
    "restore-rejection",
    "verify",
    "verify-rejection",
  ] as const)(
    "preserves %s facts and exposes a hidden receipt read failure",
    async (operation) => {
      const ports = pagePorts();
      const receipt = restoreAttemptSchema.parse(restoreAttemptSample());
      vi.mocked(ports.sessions.listRestoreAttempts).mockResolvedValue([
        receipt,
      ]);
      const view = renderPage(ports);
      await settledReads(ports);
      view.client().setQueryDefaults(["sessions-attempts"], { retry: false });
      const operationError = new Error("original operation rejected");
      const readError = new Error("persisted receipt read failed");
      ports.sessions.restoreSessionPackage = vi.fn(async () => {
        if (operation === "restore-rejection") throw operationError;
        return [receipt];
      });
      ports.sessions.verifyNativeReadback = vi.fn(async () => {
        if (operation === "verify-rejection") throw operationError;
        return receipt;
      });
      vi.mocked(ports.sessions.listRestoreAttempts).mockRejectedValue(
        readError,
      );
      view.visibility(false);
      let result: unknown;
      let error: unknown;
      await act(async () => {
        const callbacks = importDelegates();
        try {
          result = operation.startsWith("verify")
            ? await callbacks.onVerifyReadback(receipt.attemptId)
            : await callbacks.onRestore(request);
        } catch (reason) {
          error = reason;
        }
      });
      const state = view.client().getQueryState(["sessions-attempts"]);
      expect(state?.status).toBe("error");
      expect(state?.error).toBe(readError);
      expect(state?.data).toEqual([receipt]);
      expect(receipt.stage).toBe("nativeWritten");
      expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(2);
      expect(ports.sessions.listSessions).toHaveBeenCalledTimes(1);
      expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(7);
      if (operation.endsWith("rejection")) {
        expect(error).toBe(operationError);
        expect(result).toBeUndefined();
      } else {
        expect(error).toBeUndefined();
        expect(result).toEqual(operation === "verify" ? receipt : [receipt]);
      }
    },
  );

  it("replaces an in-flight receipt read after hidden restore and rejects its late stale result", async () => {
    const ports = pagePorts();
    const before = restoreAttemptSchema.parse(restoreAttemptSample());
    const after = restoreAttemptSchema.parse(
      restoreAttemptSample({ stage: "nativeReadbackVerified" }),
    );
    vi.mocked(ports.sessions.listRestoreAttempts).mockResolvedValue([before]);
    const view = renderPage(ports);
    await settledReads(ports);
    let releaseOldRead!: () => void;
    const oldReadGate = new Promise<void>((resolve) => {
      releaseOldRead = resolve;
    });
    vi.mocked(ports.sessions.listRestoreAttempts)
      .mockImplementationOnce(async () => {
        await oldReadGate;
        return [before];
      })
      .mockResolvedValue([after]);
    let oldRead!: Promise<unknown>;
    await act(async () => {
      oldRead = view
        .client()
        .fetchQuery({
          queryKey: ["sessions-attempts"],
          queryFn: () => ports.sessions.listRestoreAttempts(),
          staleTime: 0,
        })
        .catch((error: unknown) => error);
    });
    expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(2);
    let releaseRestore!: () => void;
    const restoreGate = new Promise<void>((resolve) => {
      releaseRestore = resolve;
    });
    ports.sessions.restoreSessionPackage = vi.fn(async () => {
      await restoreGate;
      return [after];
    });
    let restored!: Promise<unknown>;
    await act(async () => {
      restored = importDelegates().onRestore(request);
    });
    view.visibility(false);
    await act(async () => {
      releaseRestore();
      expect(await restored).toEqual([after]);
    });
    expect(ports.sessions.listRestoreAttempts).toHaveBeenCalledTimes(3);
    expect(view.client().getQueryData(["sessions-attempts"])).toEqual([after]);
    await act(async () => {
      releaseOldRead();
      await oldRead;
    });
    expect(view.client().getQueryData(["sessions-attempts"])).toEqual([after]);
    expect(ports.sessions.restoreSessionPackage).toHaveBeenCalledTimes(1);
    expect(ports.sessions.listSessions).toHaveBeenCalledTimes(1);
    expect(ports.sessions.probeLocalProvider).toHaveBeenCalledTimes(7);
  });
});
