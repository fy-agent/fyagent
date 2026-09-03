import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AboutSection } from "@/components/settings/AboutSection";

const mocks = vi.hoisted(() => ({
  getVersion: vi.fn(),
  getToolVersions: vi.fn(),
  probeToolInstallations: vi.fn(),
  runToolLifecycleAction: vi.fn(),
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
  toastInfo: vi.fn(),
  toastWarning: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    getToolVersions: mocks.getToolVersions,
    probeToolInstallations: mocks.probeToolInstallations,
    runToolLifecycleAction: mocks.runToolLifecycleAction,
  },
  systemApi: {
    getVersion: mocks.getVersion,
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: mocks.toastSuccess,
    error: mocks.toastError,
    info: mocks.toastInfo,
    warning: mocks.toastWarning,
  },
}));

vi.mock("@/components/settings/ToolUpgradeConfirmDialog", () => ({
  ToolUpgradeConfirmDialog: () => null,
}));

const READ_ONLY_TOOL_LABELS = [
  "Claude Code",
  "Codex CLI",
  "Gemini CLI",
  "OpenCode",
  "OpenClaw",
  "Hermes",
] as const;

const toolVersion = (name: string) => ({
  name,
  version: name === "claude" || name === "codex" ? "1.0.0" : "2.0.0",
  latest_version: name === "claude" || name === "codex" ? "2.0.0" : "2.0.0",
  error: null,
  installed_but_broken: false,
});

function toolCard(label: string): HTMLElement {
  const heading = screen
    .getAllByText(label)
    .find(
      (node) =>
        node instanceof HTMLElement &&
        node.className.includes("truncate text-sm font-medium"),
    );
  expect(heading).toBeDefined();
  const card = heading?.closest("div[class*='min-h']");
  expect(card).not.toBeNull();
  return card as HTMLElement;
}

describe("AboutSection", () => {
  let dateNow: ReturnType<typeof vi.spyOn>;
  let clock = 1_700_000_000_000;

  beforeEach(() => {
    clock += 11 * 60 * 1000;
    dateNow = vi.spyOn(Date, "now").mockReturnValue(clock);
    mocks.getVersion.mockResolvedValue("0.1.0");
    mocks.getToolVersions.mockImplementation(async (tools: string[]) =>
      tools.map(toolVersion),
    );
    mocks.probeToolInstallations.mockResolvedValue([]);
    mocks.runToolLifecycleAction.mockResolvedValue(undefined);
  });

  afterEach(() => {
    dateNow.mockRestore();
  });

  it("keeps non-Grok tools read-only and does not offer copy-command installers", async () => {
    render(<AboutSection isPortable={false} />);

    expect(
      screen.queryByRole("button", { name: "settings.manualInstallCommands" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "settings.updateAllTools" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/npm i -g/)).not.toBeInTheDocument();
    expect(screen.queryByText(/install\.sh/)).not.toBeInTheDocument();
    expect(screen.queryByText(/install\.ps1/)).not.toBeInTheDocument();
    expect(screen.queryByText(/winget/i)).not.toBeInTheDocument();

    await waitFor(() => {
      expect(
        mocks.getToolVersions.mock.calls.some(
          ([tools]) => Array.isArray(tools) && tools.includes("codex"),
        ),
      ).toBe(true);
      expect(
        mocks.getToolVersions.mock.calls.every((call) => call.length === 1),
      ).toBe(true);
    });

    for (const label of READ_ONLY_TOOL_LABELS) {
      const card = toolCard(label);
      await waitFor(() => {
        expect(
          within(card).queryByText("common.loading"),
        ).not.toBeInTheDocument();
      });
      expect(
        within(card).queryByRole("button", { name: "settings.toolInstall" }),
      ).not.toBeInTheDocument();
      expect(
        within(card).queryByRole("button", { name: "settings.toolUpdate" }),
      ).not.toBeInTheDocument();
      expect(
        within(card).queryByRole("button", {
          name: "settings.grokUseOfficialNative",
        }),
      ).not.toBeInTheDocument();
    }

    const claudeCard = toolCard("Claude Code");
    await waitFor(() => {
      expect(within(claudeCard).getByText("1.0.0")).toBeInTheDocument();
    });
    expect(
      within(claudeCard).queryByText("settings.updateAvailableShort"),
    ).not.toBeInTheDocument();

    expect(mocks.runToolLifecycleAction).not.toHaveBeenCalled();
  });

  it("lets Grok Build keep its CLI install and update actions", async () => {
    const user = userEvent.setup();
    mocks.getToolVersions.mockImplementation(async (tools: string[]) =>
      tools.map((name) =>
        name === "grok"
          ? {
              name,
              version: "1.0.0",
              latest_version: "2.0.0",
              error: null,
              installed_but_broken: false,
              distribution_owner: "official_npm",
            }
          : toolVersion(name),
      ),
    );

    render(<AboutSection isPortable={false} />);
    const card = toolCard("Grok Build");
    await waitFor(() => {
      expect(
        within(card).getByRole("button", { name: "settings.toolUpdate" }),
      ).toBeEnabled();
    });

    await user.click(
      within(card).getByRole("button", { name: "settings.toolUpdate" }),
    );

    await waitFor(() => {
      expect(mocks.probeToolInstallations).toHaveBeenCalledWith(["grok"]);
      expect(mocks.runToolLifecycleAction).toHaveBeenCalledWith(
        ["grok"],
        "update",
      );
    });
    expect(mocks.runToolLifecycleAction.mock.calls).toEqual([
      [["grok"], "update"],
    ]);
  });

  it("installs Grok via official npm by default and native only when chosen", async () => {
    const user = userEvent.setup();
    mocks.getToolVersions.mockImplementation(async (tools: string[]) =>
      tools.map((name) =>
        name === "grok"
          ? {
              name,
              version: null,
              latest_version: "1.0.13",
              error: null,
              installed_but_broken: false,
              distribution_owner: null,
              latest_source: "official_npm",
            }
          : toolVersion(name),
      ),
    );
    render(<AboutSection isPortable={false} />);
    const card = toolCard("Grok Build");
    await waitFor(() => {
      expect(
        within(card).getByRole("button", {
          name: "settings.grokUseOfficialNative",
        }),
      ).toBeEnabled();
      expect(
        within(card).getByRole("button", { name: "settings.toolInstall" }),
      ).toBeEnabled();
    });
    expect(
      within(card).getByText("settings.grokInstallNetworkNote"),
    ).toBeInTheDocument();
    await user.click(
      within(card).getByRole("button", { name: "settings.toolInstall" }),
    );
    await waitFor(() => {
      expect(mocks.runToolLifecycleAction).toHaveBeenCalledWith(
        ["grok"],
        "install",
      );
    });
    expect(mocks.runToolLifecycleAction).not.toHaveBeenCalledWith(
      ["grok"],
      "install_native",
    );
    await user.click(
      within(card).getByRole("button", {
        name: "settings.grokUseOfficialNative",
      }),
    );
    await waitFor(() => {
      expect(mocks.runToolLifecycleAction).toHaveBeenCalledWith(
        ["grok"],
        "install_native",
      );
    });
  });

  it("does not render absolute paths when diagnosing Grok conflicts", async () => {
    const user = userEvent.setup();
    const leakedPath = "C:\\Users\\alice\\AppData\\Roaming\\npm\\grok.cmd";
    mocks.probeToolInstallations.mockResolvedValue([
      {
        tool: "grok",
        is_conflict: true,
        needs_confirmation: false,
        command: "npm i -g @xai-official/grok@1.0.13 --registry=https://mirrors.tencent.com/npm/",
        anchored: true,
        installs: [
          {
            path: leakedPath,
            version: "1.0.0",
            runnable: true,
            error: null,
            source: "npm",
            is_path_default: true,
          },
        ],
      },
    ]);

    render(<AboutSection isPortable={false} />);
    const grokCard = toolCard("Grok Build");
    await waitFor(() => {
      expect(
        within(grokCard).queryByText("common.loading"),
      ).not.toBeInTheDocument();
    });
    await user.click(
      screen.getByRole("button", { name: "settings.toolDiagnose" }),
    );
    await waitFor(() => {
      expect(
        within(grokCard).getByText("settings.toolConflictTitle"),
      ).toBeInTheDocument();
    });
    expect(within(grokCard).getByText("npm")).toBeInTheDocument();
    expect(within(grokCard).getByText("1.0.0")).toBeInTheDocument();
    expect(within(grokCard).queryByText(leakedPath)).not.toBeInTheDocument();
    expect(screen.queryByText(/npm i -g/)).not.toBeInTheDocument();
  });

  it("shows the Windows administrator runtime status only in About", async () => {
    render(
      <AboutSection
        isPortable={false}
        runtimePrivilege={{
          platform: "windows",
          supported: true,
          elevated: true,
          localAdministrator: true,
          interactiveUserMatch: "match",
        }}
      />,
    );

    expect(
      screen.getByText("settings.runtimePrivilegeAdministrator"),
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(mocks.getVersion).toHaveBeenCalled();
    });
  });
});
