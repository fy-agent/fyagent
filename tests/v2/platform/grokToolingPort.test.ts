import { beforeEach, describe, expect, it, vi } from "vitest";

import { createGrokToolingPort } from "@/v2/shared/platform/tauri/feature-ports/grokTooling";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Tauri Grok tooling port", () => {
  beforeEach(() => invoke.mockReset());

  it("reads grok versions through the existing command and installs official npm without a fifth command", async () => {
    invoke.mockResolvedValueOnce([
      {
        name: "grok",
        version: "1.0.5",
        latest_version: "1.0.6",
        error: null,
        installed_but_broken: false,
        distribution_owner: "native_internal",
        latest_source: "native_internal",
      },
    ]);
    invoke.mockResolvedValueOnce(undefined);
    const port = createGrokToolingPort();
    await expect(port.getSnapshot()).resolves.toMatchObject({
      distributionOwner: "native_internal",
      latestSource: "native_internal",
    });
    await port.installOfficialNpm();
    expect(invoke.mock.calls).toEqual([
      ["get_tool_versions", { tools: ["grok"] }],
      [
        "run_tool_lifecycle_action",
        { tools: ["grok"], action: "install_official_npm" },
      ],
    ]);
  });
});
