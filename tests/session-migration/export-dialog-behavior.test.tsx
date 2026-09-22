import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  ExportPreviewDialog,
  type ExportTargetItem,
} from "@/pages/sessions/components/ExportPreviewDialog";
import {
  migratableSessionSchema,
  type MigratableSession,
} from "@/shared/features/session-migration";
import { sessionPackageSample } from "./samples";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

const targets: ExportTargetItem[] = [
  {
    providerId: "codex",
    sourcePath: "/isolated/session.jsonl",
    sessionId: "session-a",
    title: "会话 A",
  },
];

function previewSession(): MigratableSession {
  const raw = structuredClone(sessionPackageSample());
  return migratableSessionSchema.parse(
    (raw.sessions as Array<Record<string, unknown>>)[0],
  );
}

function orderedUserPreview(): MigratableSession {
  const raw = structuredClone(sessionPackageSample());
  const session = (raw.sessions as Array<Record<string, unknown>>)[0];
  session.messages = [
    { seq: 0, kind: "userText", text: "A：未获得答复。" },
    { seq: 1, kind: "userText", text: "B：重新提问。" },
    { seq: 2, kind: "assistantFinal", text: "B 的最终答复。" },
    { seq: 3, kind: "userText", text: "C：尾部未完成。" },
  ];
  const extraction = session.extraction as Record<string, unknown>;
  extraction.openUserMessages = [0, 3];
  return migratableSessionSchema.parse(session);
}

function dialogProps(
  onPreviewSession: (
    providerId: string,
    sourcePath: string,
  ) => Promise<MigratableSession>,
) {
  return {
    open: true,
    onOpenChange: vi.fn(),
    targets,
    onPreviewSession,
    onExport: vi.fn(async (_items, targetPath: string) => ({
      path: targetPath,
      sessionCount: 1,
      byteLen: 128,
      packageFileDigest: "a".repeat(64),
    })),
    onPickExportPath: vi.fn(async () => "/tmp/export.json"),
    originRef: undefined,
  };
}

describe("session export preview behavior", () => {
  it("leaves ready state immediately when preview validation re-enters", async () => {
    const user = userEvent.setup();
    const initialPreview = vi.fn(async () => previewSession());
    const initialProps = dialogProps(initialPreview);
    const view = render(<ExportPreviewDialog {...initialProps} />);

    const dialog = await screen.findByRole("dialog");
    await waitFor(() =>
      expect(
        within(dialog).getByRole("button", { name: "选择保存位置" }),
      ).toBeEnabled(),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "选择保存位置" }),
    );
    expect(
      within(dialog).getByRole("button", { name: "确认导出迁移包" }),
    ).toBeEnabled();

    const revalidation = deferred<MigratableSession>();
    const nextProps = {
      ...initialProps,
      onPreviewSession: vi.fn(() => revalidation.promise),
    };
    view.rerender(<ExportPreviewDialog {...nextProps} />);

    await waitFor(() =>
      expect(
        within(dialog).getByRole("button", { name: "确认导出迁移包" }),
      ).toBeDisabled(),
    );
    expect(within(dialog).getAllByText(/正在读取/u).length).toBeGreaterThan(0);
    expect(initialProps.onExport).not.toHaveBeenCalled();
  });

  it("previews consecutive users as ordered messages with incomplete state", async () => {
    render(
      <ExportPreviewDialog
        {...dialogProps(async () => orderedUserPreview())}
      />,
    );

    const dialog = await screen.findByRole("dialog");
    const preview = await waitFor(() => {
      const element = dialog.querySelector(".fy-pure-text-box");
      expect(element).not.toBeNull();
      expect(element?.textContent).toContain("A：未获得答复。");
      return element as HTMLElement;
    });
    const text = preview.textContent ?? "";

    const bodies = [
      "A：未获得答复。",
      "B：重新提问。",
      "B 的最终答复。",
      "C：尾部未完成。",
    ];
    expect(bodies.map((body) => text.indexOf(body))).toEqual(
      [...bodies.map((body) => text.indexOf(body))].sort((a, b) => a - b),
    );
    for (const body of bodies) {
      expect(text.split(body)).toHaveLength(2);
    }
    expect(text.match(/未完成/gu)?.length ?? 0).toBeGreaterThanOrEqual(2);
  });
});
