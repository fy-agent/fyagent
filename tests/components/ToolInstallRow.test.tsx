import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ToolInstallRow } from "@/components/settings/ToolInstallRow";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("ToolInstallRow", () => {
  it("shows source and version without the absolute path", () => {
    render(
      <ToolInstallRow
        inst={{
          path: "C:\\Users\\alice\\AppData\\Roaming\\npm\\grok.cmd",
          version: "1.2.3",
          runnable: true,
          error: null,
          source: "npm",
          is_path_default: true,
        }}
      />,
    );

    expect(screen.getByText("npm")).toBeInTheDocument();
    expect(screen.getByText("1.2.3")).toBeInTheDocument();
    expect(
      screen.getByText("settings.toolConflictDefault"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/C:\\Users\\alice/)).not.toBeInTheDocument();
  });
});
