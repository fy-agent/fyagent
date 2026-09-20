import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../fixtures/configPackDtoContract.v1.json";
import { createConfigPackPort } from "@/shared/platform/tauri/feature-ports/configPack";
import { parseImportPreview } from "@/domain/config-pack";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const preview = parseImportPreview({
  previewId: "11111111-1111-4111-8111-111111111111",
  digest: "a".repeat(64),
  entries: fixture.providers.map((provider) => ({
    provider,
    action: "add",
    conflict: false,
    canOverwrite: false,
    existing: null,
    credentialsRequired: true,
  })),
});
describe("configuration pack native port", () => {
  beforeEach(() => {
    invoke.mockReset();
  });
  it("confirms only a preview capability and validates real readback", async () => {
    const port = createConfigPackPort();
    invoke.mockResolvedValue(preview);
    await port.previewImport(JSON.stringify(fixture), []);
    expect(invoke).toHaveBeenLastCalledWith("preview_config_pack_import", {
      request: { text: JSON.stringify(fixture), choices: [] },
    });
    invoke.mockResolvedValue({ providers: fixture.providers, skipped: 0 });
    await expect(port.apply(preview)).resolves.toEqual({
      providers: fixture.providers,
      skipped: 0,
    });
    expect(invoke).toHaveBeenLastCalledWith("apply_config_pack_import", {
      request: { previewId: preview.previewId, digest: preview.digest },
    });
    invoke.mockResolvedValue({ providers: [], skipped: 2 });
    await expect(port.apply(preview)).rejects.toThrow("无法确认保存结果");
  });
  it("uses native file pickers without a caller path and preserves cancellation", async () => {
    const port = createConfigPackPort();
    invoke.mockResolvedValue(null);
    await expect(port.pickFile()).resolves.toBeNull();
    expect(invoke).toHaveBeenLastCalledWith("pick_config_pack_file");
    const output = {
      exportId: preview.previewId,
      digest: preview.digest,
      text: JSON.stringify(fixture),
    };
    invoke.mockResolvedValue(false);
    await expect(port.saveExport(output)).resolves.toBe(false);
    expect(invoke).toHaveBeenLastCalledWith("save_config_pack_export", {
      request: { previewId: preview.previewId, digest: preview.digest },
    });
    invoke.mockResolvedValue({ text: "not a string" });
    await expect(port.pickFile()).rejects.toThrow();
  });
  it("rejects malformed native replies and never renders raw secret errors", async () => {
    const port = createConfigPackPort();
    invoke.mockResolvedValue({
      entries: [],
      excluded: 0,
      token: "SECRET-CANARY",
    });
    await expect(port.list()).rejects.not.toThrow("CANARY");
    invoke.mockRejectedValue("SECRET-CANARY /Users/private");
    await expect(port.list()).rejects.not.toThrow("CANARY");
    invoke.mockRejectedValue("stale_preview");
    await expect(port.apply(preview)).rejects.toThrow("重新预览");
    await expect(
      createBrowserFeaturePorts().configPack.apply(preview),
    ).rejects.toThrow("桌面应用");
  });
});
