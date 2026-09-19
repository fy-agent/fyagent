import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { createVerificationPort } from "@/shared/platform/tauri/feature-ports/verification";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import {
  PROJECT,
  OTHER_PROJECT,
  verificationFixture,
} from "../fixtures/verification";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => {
  vi.mocked(invoke).mockReset();
});
describe("verification native port", () => {
  it("sends only a saved project identity to checks and rejects cross-project results", async () => {
    vi.mocked(invoke).mockResolvedValue(verificationFixture());
    const port = createVerificationPort();
    await port.run({
      projectId: PROJECT,
      expectedRevision: 1,
      checker: "saved_model_probe",
      runId: OTHER_PROJECT,
      fixture: null,
    });
    expect(invoke).toHaveBeenCalledWith("run_project_verification", {
      request: {
        projectId: PROJECT,
        expectedRevision: 1,
        checker: "saved_model_probe",
      runId: OTHER_PROJECT,
      fixture: null,
      },
    });
    vi.mocked(invoke).mockResolvedValue(verificationFixture(OTHER_PROJECT));
    await expect(port.get(PROJECT)).rejects.toThrow("无法确认");
  });
  it("requires matching parsed JSON preview and safe native-only failures", async () => {
    const snapshot = verificationFixture();
    vi.mocked(invoke).mockResolvedValue({
      snapshot,
      json: JSON.stringify(snapshot),
      markdown: "# 交接",
    });
    await expect(
      createVerificationPort().preview(PROJECT),
    ).resolves.toMatchObject({ snapshot });
    vi.mocked(invoke).mockResolvedValue({
      snapshot,
      json: JSON.stringify(verificationFixture(OTHER_PROJECT)),
      markdown: "# 交接",
    });
    await expect(createVerificationPort().preview(PROJECT)).rejects.toThrow(
      "无法确认",
    );
    await expect(
      createBrowserFeaturePorts().verification.get(PROJECT),
    ).rejects.toThrow("桌面应用");
  });
  it("preserves cancelled export and hides native diagnostics", async () => {
    vi.mocked(invoke).mockResolvedValue(false);
    await expect(
      createVerificationPort().export(PROJECT, "json"),
    ).resolves.toBe(false);
    vi.mocked(invoke).mockRejectedValue("sk-secret-canary");
    await expect(createVerificationPort().get(PROJECT)).rejects.toThrow(
      "无法确认",
    );
  });
});
