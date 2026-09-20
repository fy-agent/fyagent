import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../fixtures/deliveryKitContract.v1.json";
import { createDeliveryKitsPort } from "@/shared/platform/tauri/feature-ports/delivery-kits";
import { parseKitPreview } from "@/domain/delivery-kits";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const preview = parseKitPreview({
  previewId: "11111111-1111-4111-8111-111111111111",
  kind: "import",
  kit: fixture,
  conflict: false,
});
describe("delivery kit native port", () => {
  beforeEach(() => invoke.mockReset());
  it("passes only identity and preview tokens, never file paths or manifest on apply", async () => {
    const port = createDeliveryKitsPort();
    invoke.mockResolvedValue(preview);
    await port.previewBuiltin(fixture.identity);
    expect(invoke).toHaveBeenLastCalledWith("preview_builtin_delivery_kit", {
      identity: fixture.identity,
    });
    invoke.mockResolvedValue({ ...fixture, installed: true });
    await port.apply(preview);
    expect(invoke).toHaveBeenLastCalledWith("apply_delivery_kit_import", {
      previewId: preview.previewId,
      manifestDigest: fixture.identity.manifestDigest,
    });
    invoke.mockResolvedValue(null);
    expect(await port.pickImport()).toBeNull();
    expect(invoke).toHaveBeenLastCalledWith("pick_delivery_kit_import");
  });
  it("rejects malformed native replies and redacts arbitrary failures", async () => {
    const port = createDeliveryKitsPort();
    invoke.mockResolvedValue([{ ...fixture, credentials: "SECRET-CANARY" }]);
    await expect(port.list()).rejects.not.toThrow("CANARY");
    invoke.mockRejectedValue("backend SECRET-CANARY");
    await expect(port.list()).rejects.toThrow("无法读取交付包");
    invoke.mockResolvedValue({ success: true });
    await expect(port.saveExport(preview)).rejects.toThrow();
  });
});
