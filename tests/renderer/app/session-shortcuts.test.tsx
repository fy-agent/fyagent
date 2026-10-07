import {
  act,
  createEvent,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import {
  createMemoryRouter,
  RouterProvider,
  useNavigate,
} from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { PersistentPrimaryOutlet } from "@/app/PersistentPrimaryOutlet";
import { FeatureProvider } from "@/shared/features/provider";
import type {
  MigratableSession,
  SessionMeta,
} from "@/shared/features/session-migration";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { Dialog } from "@/shared/ui/Dialog";

// Actual production session page, dialogs and keep-alive outlet; only the
// neighboring route and data/OS boundaries are controlled renderer fixtures.
vi.mock("@/shared/platform/runtime", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/shared/platform/runtime")>()),
  detectRuntime: () => ({ isNative: true, platform: "windows" }),
}));
vi.mock("@/shared/platform/useFrontendReady", () => ({
  useFrontendReady: () => undefined,
}));
vi.mock("@/app/primaryPages", async () => {
  const { SessionsPage } = await import("@/pages/sessions/Page");
  return {
    primaryPages: {
      sessions: SessionsPage,
      models: () => <input aria-label="模型页输入" defaultValue="模型草稿" />,
    },
  };
});

const sessions: SessionMeta[] = [
  {
    providerId: "codex",
    sessionId: "fixture-alpha",
    title: "Alpha 会话",
    sourcePath: "C:/fixture/alpha.jsonl",
    projectDir: "C:/fixture/project",
  },
  {
    providerId: "codex",
    sessionId: "fixture-beta",
    title: "Beta 会话",
    sourcePath: "C:/fixture/beta.jsonl",
    projectDir: "C:/fixture/project",
  },
];

function previewSession(sourcePath: string): MigratableSession {
  return {
    snapshotId: sourcePath,
    contentDigest: "fixture-digest",
    origin: {
      originId: "fixture-origin",
      providerId: "codex",
      sessionId: sourcePath,
    },
    title: sourcePath.includes("alpha") ? "Alpha 会话" : "Beta 会话",
    messages: [
      { seq: 0, kind: "userText", text: "fixture question" },
      { seq: 1, kind: "assistantFinal", text: "fixture answer" },
    ],
    extraction: {
      ruleId: "fixture-rule",
      ruleVerifiedVersions: ["fixture-version"],
      openUserMessages: [],
      omitted: {
        toolEvents: 0,
        reasoningBlocks: 0,
        commentaryMessages: 0,
        attachments: 0,
        runtimeInjections: 0,
        unknownBlocks: 0,
      },
      sourcePathFamily: "windows",
    },
  };
}

function fixturePorts() {
  const ports = createBrowserFeaturePorts();
  ports.sessions.listSessions = vi.fn(async () => sessions);
  ports.sessions.listRestoreAttempts = vi.fn(async () => []);
  ports.sessions.probeLocalProvider = vi.fn(async (providerId) => ({
    providerId,
    installed: true,
    extractionSupported: true,
    writeSupported: true,
  }));
  ports.sessions.getSessionMessages = vi.fn(async () => []);
  ports.sessions.previewSessionMigration = vi.fn(
    async (_providerId, sourcePath) => previewSession(sourcePath),
  );
  ports.sessions.readSessionPackage = vi.fn(async () => ({
    package: {
      schema: "fyagent.session.v1",
      exportedAt: 1,
      exporter: { app: "FyAgent", appVersion: "fixture", platform: "windows" },
      sessions: [previewSession(sessions[0].sourcePath!)],
    },
    attempts: [],
  }));
  ports.sessions.restoreSessionPackage = vi.fn(async () => []);
  ports.sessions.exportSessionPackage = vi.fn(async (_items, targetPath) => ({
    path: targetPath,
    sessionCount: 1,
    byteLen: 1,
    packageFileDigest: "fixture",
  }));
  ports.sessions.pickPackageFile = vi.fn(async () => null);
  ports.sessions.pickExportPath = vi.fn(async () => null);
  ports.sessions.pickDirectory = vi.fn(async () => null);
  return ports;
}

async function renderSessions({
  externalDialog = false,
  ancestorActive = true,
} = {}) {
  const ports = fixturePorts();
  function Shell() {
    const navigate = useNavigate();
    return (
      <>
        <button onClick={() => void navigate("/models")}>切到模型</button>
        <button onClick={() => void navigate("/sessions")}>切回会话</button>
        <PersistentPrimaryOutlet />
        <Dialog
          open={externalDialog}
          onOpenChange={() => undefined}
          title="壳层弹窗"
          originRef={undefined}
        >
          <button>弹窗操作</button>
        </Dialog>
      </>
    );
  }
  const router = createMemoryRouter([{ path: "*", element: <Shell /> }], {
    initialEntries: ["/sessions"],
  });
  const { PersistentSurface } = await import("@/shared/ui/PersistentSurface");
  const view = render(
    <FeatureProvider ports={ports}>
      <PersistentSurface active={ancestorActive}>
        <RouterProvider router={router} />
      </PersistentSurface>
    </FeatureProvider>,
  );
  await waitFor(() =>
    expect(ports.sessions.listSessions).toHaveBeenCalledOnce(),
  );
  // Waiting for fixture content also flushes the page's initial data renders.
  await screen.findByText("Alpha 会话");
  return { ...view, ports, router };
}

function shortcut(
  target: Element | Window,
  key: "i" | "e",
  modifier: "ctrlKey" | "metaKey" = "ctrlKey",
  extra: KeyboardEventInit = {},
) {
  const event = createEvent.keyDown(target, {
    key,
    [modifier]: true,
    bubbles: true,
    cancelable: true,
    ...extra,
  });
  fireEvent(target, event);
  return event;
}

function expectNoWrites(ports: ReturnType<typeof fixturePorts>) {
  expect(ports.sessions.restoreSessionPackage).not.toHaveBeenCalled();
  expect(ports.sessions.exportSessionPackage).not.toHaveBeenCalled();
  expect(ports.sessions.pickPackageFile).not.toHaveBeenCalled();
  expect(ports.sessions.pickExportPath).not.toHaveBeenCalled();
  expect(ports.sessions.pickDirectory).not.toHaveBeenCalled();
}

async function selectAlpha() {
  fireEvent.click(screen.getByText("Alpha 会话"));
  await screen.findByText("fixture question");
}

describe("session shortcuts on the production keep-alive page", () => {
  it.each(["ctrlKey", "metaKey"] as const)(
    "ignores hidden I/E and keeps selection/search on return (%s)",
    async (modifier) => {
      const { ports } = await renderSessions();
      await selectAlpha();
      fireEvent.change(screen.getByRole("searchbox", { name: "搜索会话" }), {
        target: { value: "Alpha" },
      });
      const originalPage = screen.getByTestId("sessions-page");
      fireEvent.click(screen.getByRole("button", { name: "切到模型" }));
      const modelInput = await screen.findByRole("textbox", {
        name: "模型页输入",
      });
      expect(originalPage).not.toBeVisible();
      expect(shortcut(document.body, "i", modifier).defaultPrevented).toBe(
        false,
      );
      expect(shortcut(document.body, "e", modifier).defaultPrevented).toBe(
        false,
      );
      modelInput.focus();
      expect(shortcut(modelInput, "i", modifier).defaultPrevented).toBe(false);
      expect(shortcut(modelInput, "e", modifier).defaultPrevented).toBe(false);
      expect(modelInput).toHaveValue("模型草稿");
      fireEvent.click(screen.getByRole("button", { name: "切回会话" }));
      await waitFor(() => expect(originalPage).toBeVisible());
      expect(screen.getByTestId("sessions-page")).toBe(originalPage);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.getByRole("searchbox", { name: "搜索会话" })).toHaveValue(
        "Alpha",
      );
      expect(
        screen.getByRole("button", { name: "导出迁移包 (⌘E)" }),
      ).toBeEnabled();
      expect(screen.getByText("fixture question")).toBeVisible();
      expectNoWrites(ports);
    },
  );

  it.each(["ctrlKey", "metaKey"] as const)(
    "opens the existing import dialog without a file action (%s)",
    async (modifier) => {
      const { ports } = await renderSessions();
      expect(
        shortcut(document.body, "i", modifier).defaultPrevented,
        document.activeElement?.tagName,
      ).toBe(true);
      expect(
        await screen.findByRole("dialog", { name: "导入跨设备会话包" }),
      ).toBeVisible();
      expect(ports.sessions.readSessionPackage).not.toHaveBeenCalled();
      expectNoWrites(ports);
    },
  );

  it.each(["ctrlKey", "metaKey"] as const)(
    "opens the existing export preview for the selected session (%s)",
    async (modifier) => {
      const { ports } = await renderSessions();
      await selectAlpha();
      const pageButton = screen.getByRole("button", {
        name: "导出迁移包 (⌘E)",
      });
      pageButton.focus();
      expect(shortcut(pageButton, "e", modifier).defaultPrevented).toBe(true);
      const dialog = await screen.findByRole("dialog", {
        name: "导出迁移包 (纯文本 Q&A 问答)",
      });
      await within(dialog).findByText(/fixture question/);
      expect(ports.sessions.previewSessionMigration).toHaveBeenCalledWith(
        "codex",
        "C:/fixture/alpha.jsonl",
      );
      expect(
        within(dialog).getByRole("button", { name: "确认导出迁移包" }),
      ).toBeDisabled();
      expectNoWrites(ports);
    },
  );

  it.each(["i", "e"] as const)(
    "respects the actual session search input for %s",
    async (key) => {
      const { ports } = await renderSessions();
      await selectAlpha();
      const input = screen.getByRole("searchbox", { name: "搜索会话" });
      input.focus();
      expect(shortcut(input, key).defaultPrevented).toBe(false);
      // Body-targeted events still honor the real active input.
      expect(shortcut(document.body, key).defaultPrevented).toBe(false);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expectNoWrites(ports);
    },
  );

  it.each([
    "textarea",
    "select",
    "contenteditable",
    "nested-contenteditable",
    "plaintext-only",
  ])("respects %s editing contexts for both shortcuts", async (kind) => {
    const { ports } = await renderSessions();
    const root = screen.getByTestId("sessions-page");
    const control = document.createElement(
      kind === "textarea" || kind === "select" ? kind : "div",
    );
    control.tabIndex = 0;
    if (kind.includes("contenteditable") || kind === "plaintext-only")
      control.setAttribute(
        "contenteditable",
        kind === "plaintext-only" ? "plaintext-only" : "true",
      );
    root.append(control);
    const target =
      kind === "nested-contenteditable"
        ? control.appendChild(document.createElement("span"))
        : control;
    control.focus();
    expect(shortcut(target, "i").defaultPrevented).toBe(false);
    expect(shortcut(target, "e").defaultPrevented).toBe(false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expectNoWrites(ports);
  });

  it.each(["i", "e"] as const)(
    "leaves a previously consumed %s event alone",
    async (key) => {
      const { ports } = await renderSessions();
      const event = createEvent.keyDown(document.body, {
        key,
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      });
      event.preventDefault();
      fireEvent(document.body, event);
      expect(event.defaultPrevented).toBe(true);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.queryByText("请先选择会话")).not.toBeInTheDocument();
      expectNoWrites(ports);
    },
  );

  it("does not open a second dialog or reset an unsaved import path", async () => {
    const { ports, router } = await renderSessions();
    const importButton = screen.getByRole("button", {
      name: "导入会话包 (⌘I)",
    });
    importButton.focus();
    shortcut(importButton, "i");
    const dialog = await screen.findByRole("dialog", {
      name: "导入跨设备会话包",
    });
    const path = within(dialog).getByPlaceholderText(
      "/path/to/session-package.json",
    );
    fireEvent.change(path, {
      target: { value: "C:/fixture/unsaved-package.json" },
    });
    // Even a modal button/non-input target belongs to the modal, not the page.
    const cancel = within(dialog).getByRole("button", { name: "取消" });
    cancel.focus();
    expect(shortcut(cancel, "i").defaultPrevented).toBe(false);
    expect(shortcut(cancel, "e").defaultPrevented).toBe(false);
    expect(shortcut(document.body, "i").defaultPrevented).toBe(false);
    expect(screen.getAllByRole("dialog")).toHaveLength(1);
    expect(path).toHaveValue("C:/fixture/unsaved-package.json");
    await act(async () => {
      await router.navigate("/models");
    });
    await screen.findByRole("textbox", { name: "模型页输入" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(shortcut(document.body, "e").defaultPrevented).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "切回会话" }));
    const resumed = await screen.findByRole("dialog", {
      name: "导入跨设备会话包",
    });
    expect(
      within(resumed).getByPlaceholderText("/path/to/session-package.json"),
    ).toHaveValue("C:/fixture/unsaved-package.json");
    expectNoWrites(ports);
  });

  it("yields to an unrelated portaled shell dialog, including body-targeted events", async () => {
    const { ports } = await renderSessions({ externalDialog: true });
    const dialog = screen.getByRole("dialog", { name: "壳层弹窗" });
    const control = within(dialog).getByRole("button", { name: "弹窗操作" });
    control.focus();
    expect(shortcut(control, "i").defaultPrevented).toBe(false);
    expect(shortcut(control, "e").defaultPrevented).toBe(false);
    expect(shortcut(document.body, "i").defaultPrevented).toBe(false);
    expect(shortcut(document.body, "e").defaultPrevented).toBe(false);
    expect(screen.getAllByRole("dialog")).toEqual([dialog]);
    control.blur();
    expect(document.activeElement).toBe(document.body);
    expect(shortcut(document.body, "i").defaultPrevented).toBe(false);
    expect(shortcut(document.body, "e").defaultPrevented).toBe(false);
    expect(screen.getAllByRole("dialog")).toEqual([dialog]);
    expectNoWrites(ports);
  });

  it("honors an inactive persistent ancestor even when sessions is the current route", async () => {
    const { ports } = await renderSessions({ ancestorActive: false });
    expect(screen.getByTestId("sessions-page")).not.toBeVisible();
    expect(shortcut(document.body, "i").defaultPrevented).toBe(false);
    expect(shortcut(document.body, "e").defaultPrevented).toBe(false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expectNoWrites(ports);
  });

  it.each([
    { repeat: true },
    { isComposing: true },
    { altKey: true },
    { shiftKey: true },
  ])("leaves other keyboard context %j alone", async (extra) => {
    const { ports } = await renderSessions();
    expect(
      shortcut(document.body, "i", "ctrlKey", extra).defaultPrevented,
    ).toBe(false);
    expect(
      shortcut(document.body, "e", "ctrlKey", extra).defaultPrevented,
    ).toBe(false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expectNoWrites(ports);
  });
});
