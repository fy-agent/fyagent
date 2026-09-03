import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ToolUpgradeConfirmDialog } from "@/components/settings/ToolUpgradeConfirmDialog";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: { open: boolean; children: React.ReactNode }) =>
    open ? <div>{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogDescription: ({ children }: { children: React.ReactNode }) => (
    <p>{children}</p>
  ),
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => (
    <h2>{children}</h2>
  ),
}));

describe("ToolUpgradeConfirmDialog", () => {
  it("does not show the upgrade command or absolute paths", () => {
    render(
      <ToolUpgradeConfirmDialog
        isOpen
        displayName={(tool) => tool}
        onConfirm={() => undefined}
        onCancel={() => undefined}
        plans={[
          {
            tool: "grok",
            is_conflict: true,
            needs_confirmation: true,
            command: "npm i -g @xai-official/grok@1.0.13 --registry=https://mirrors.tencent.com/npm/",
            anchored: true,
            installs: [
              {
                path: "/Users/alice/.local/bin/grok",
                version: "1.0.0",
                runnable: true,
                error: null,
                source: "path",
                is_path_default: true,
              },
            ],
          },
        ]}
      />,
    );

    expect(screen.getByText("grok")).toBeInTheDocument();
    expect(screen.getByText("path")).toBeInTheDocument();
    expect(screen.getByText("1.0.0")).toBeInTheDocument();
    expect(
      screen.queryByText("settings.toolUpgradeWillRun"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/npm i -g/)).not.toBeInTheDocument();
    expect(screen.queryByText(/\/Users\/alice/)).not.toBeInTheDocument();
  });
});
