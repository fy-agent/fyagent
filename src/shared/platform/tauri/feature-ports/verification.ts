import { invoke } from "@tauri-apps/api/core";
import {
  projectIdSchema,
  snapshotSchema,
  runSchema,
  manualSchema,
  revokeSchema,
  saveHandoffSchema,
  previewSchema,
  type VerificationPort,
} from "@/domain/verification";

const failure = "无法确认验证结果，请刷新后重试";
export function createVerificationPort(): VerificationPort {
  async function snapshot(read: () => Promise<unknown>, projectId: string) {
    try {
      const parsed = snapshotSchema.parse(await read());
      if (parsed.projectId !== projectId) throw new Error(failure);
      return parsed;
    } catch {
      throw new Error(failure);
    }
  }
  return {
    cancel: async (projectId) => {
      try {
        const result = await invoke<unknown>("cancel_project_verification", {
          projectId: projectIdSchema.parse(projectId),
        });
        if (typeof result !== "boolean") throw new Error(failure);
        return result;
      } catch {
        throw new Error(failure);
      }
    },
    get: (projectId) =>
      snapshot(
        () =>
          invoke<unknown>("get_project_verification", {
            projectId: projectIdSchema.parse(projectId),
          }),
        projectId,
      ),
    run: (request) =>
      snapshot(
        () =>
          invoke<unknown>("run_project_verification", {
            request: runSchema.parse(request),
          }),
        request.projectId,
      ),
    record: (request) =>
      snapshot(
        () =>
          invoke<unknown>("record_project_verification", {
            request: manualSchema.parse(request),
          }),
        request.projectId,
      ),
    revoke: (request) =>
      snapshot(
        () =>
          invoke<unknown>("revoke_project_verification", {
            request: revokeSchema.parse(request),
          }),
        request.projectId,
      ),
    saveHandoff: (request) =>
      snapshot(
        () =>
          invoke<unknown>("save_project_handoff", {
            request: saveHandoffSchema.parse(request),
          }),
        request.projectId,
      ),
    preview: async (projectId) => {
      try {
        const result = previewSchema.parse(
          await invoke<unknown>("preview_project_handoff", {
            projectId: projectIdSchema.parse(projectId),
          }),
        );
        if (
          result.snapshot.projectId !== projectId ||
          JSON.stringify(snapshotSchema.parse(JSON.parse(result.json))) !==
            JSON.stringify(result.snapshot)
        )
          throw new Error(failure);
        return result;
      } catch {
        throw new Error(failure);
      }
    },
    export: async (projectId, format) => {
      projectIdSchema.parse(projectId);
      if (format !== "json" && format !== "markdown") throw new Error(failure);
      try {
        const result = await invoke<unknown>("export_project_handoff", {
          projectId,
          format,
        });
        if (typeof result !== "boolean") throw new Error(failure);
        return result;
      } catch {
        throw new Error("导出未完成，请重试");
      }
    },
  };
}
