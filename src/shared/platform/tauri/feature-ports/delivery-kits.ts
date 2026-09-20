import { invoke } from "@tauri-apps/api/core";
import {
  assertPreviewRequest,
  parseKitDemo,
  parseKitIdentity,
  parseKitList,
  parseKitPreview,
  parseKitView,
  type KitIdentity,
} from "../../../../domain/delivery-kits";
import {
  safeKitError,
  type DeliveryKitsPort,
} from "../../../features/delivery-kits";

async function safe<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (e) {
    throw safeKitError(e);
  }
}

function checkedPreview(
  value: unknown,
  kind: "import" | "export",
  expected?: KitIdentity,
) {
  const preview = parseKitPreview(value);
  if (
    preview.kind !== kind ||
    (expected &&
      (preview.kit.identity.kitId !== expected.kitId ||
        preview.kit.identity.kitVersion !== expected.kitVersion ||
        preview.kit.identity.manifestDigest !== expected.manifestDigest))
  )
    throw new Error("invalid preview response");
  return preview;
}

// Deliberately exported as an independent port: Projects composes the panel and root
// merges feature-facade wiring without a second top-level route.
export function createDeliveryKitsPort(): DeliveryKitsPort {
  return {
    list: () =>
      safe(async () =>
        parseKitList(await invoke<unknown>("list_delivery_kits")),
      ),
    previewBuiltin: (identity) =>
      safe(async () =>
        checkedPreview(
          await invoke<unknown>("preview_builtin_delivery_kit", {
            identity: parseKitIdentity(identity),
          }),
          "import",
          identity,
        ),
      ),
    pickImport: () =>
      safe(async () => {
        const result = await invoke<unknown>("pick_delivery_kit_import");
        return result === null ? null : checkedPreview(result, "import");
      }),
    apply: (preview) =>
      safe(async () => {
        const view = parseKitView(
          await invoke<unknown>(
            "apply_delivery_kit_import",
            assertPreviewRequest(
              preview.previewId,
              preview.kit.identity.manifestDigest,
            ),
          ),
        );
        if (
          !view.installed ||
          view.identity.kitId !== preview.kit.identity.kitId ||
          view.identity.kitVersion !== preview.kit.identity.kitVersion ||
          view.identity.manifestDigest !== preview.kit.identity.manifestDigest
        )
          throw new Error("invalid import result");
        return view;
      }),
    cancel: (preview) =>
      safe(async () => {
        const request = assertPreviewRequest(
          preview.previewId,
          preview.kit.identity.manifestDigest,
        );
        const result = await invoke<unknown>("cancel_delivery_kit_preview", {
          previewId: request.previewId,
        });
        if (result !== null) throw new Error("invalid result");
      }),
    previewExport: (identity) =>
      safe(async () =>
        checkedPreview(
          await invoke<unknown>("preview_delivery_kit_export", {
            identity: parseKitIdentity(identity),
          }),
          "export",
          identity,
        ),
      ),
    saveExport: (preview) =>
      safe(async () => {
        const result = await invoke<unknown>(
          "save_delivery_kit_export",
          assertPreviewRequest(
            preview.previewId,
            preview.kit.identity.manifestDigest,
          ),
        );
        if (typeof result !== "boolean") throw new Error("invalid result");
        return result;
      }),
    runDemo: (identity) =>
      safe(async () => {
        const checked = parseKitIdentity(identity);
        return parseKitDemo(
          await invoke<unknown>("run_delivery_kit_demo", { identity: checked }),
          checked,
        );
      }),
  };
}
