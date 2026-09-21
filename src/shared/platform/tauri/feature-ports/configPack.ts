import { invoke } from "@tauri-apps/api/core";
import {
  confirmRequest,
  importRequest,
  parseConfigPack,
  parseExportPreview,
  parseImportPreview,
  parseImportResult,
  parsePackCandidates,
  parseSelection,
} from "../../../../domain/config-pack";
import {
  safePackError,
  type ConfigPackPort,
} from "../../../features/config-pack";

async function safe<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (e) {
    throw safePackError(e);
  }
}
export function createConfigPackPort(): ConfigPackPort {
  return {
    list: () =>
      safe(async () =>
        parsePackCandidates(
          await invoke<unknown>("list_config_pack_candidates"),
        ),
      ),
    pickFile: () =>
      safe(async () => {
        const result = await invoke<unknown>("pick_config_pack_file");
        if (result === null) return null;
        if (typeof result !== "string") throw new Error("invalid file result");
        parseConfigPack(result);
        return result;
      }),
    previewImport: (text, choices) =>
      safe(async () =>
        parseImportPreview(
          await invoke<unknown>("preview_config_pack_import", {
            request: importRequest(text, choices),
          }),
        ),
      ),
    apply: (preview) =>
      safe(async () => {
        const checked = parseImportPreview(preview);
        return parseImportResult(
          await invoke<unknown>("apply_config_pack_import", {
            request: confirmRequest(checked.previewId, checked.digest),
          }),
          checked,
        );
      }),
    previewExport: (selection) =>
      safe(async () =>
        parseExportPreview(
          await invoke<unknown>("preview_config_pack_export", {
            selection: parseSelection(selection),
          }),
        ),
      ),
    saveExport: (preview) =>
      safe(async () => {
        const checked = parseExportPreview(preview);
        const result = await invoke<unknown>("save_config_pack_export", {
          request: confirmRequest(checked.exportId, checked.digest),
        });
        if (typeof result !== "boolean")
          throw new Error("invalid export result");
        return result;
      }),
    cancel: (previewId) =>
      safe(async () => {
        const result = await invoke<unknown>("cancel_config_pack_preview", {
          previewId,
        });
        if (result !== null) throw new Error("invalid cancel result");
      }),
  };
}
