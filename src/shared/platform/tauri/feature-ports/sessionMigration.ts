import { invoke } from "@tauri-apps/api/core";
import * as z from "zod/mini";

import {
  localProviderProbeSchema,
  migratableSessionSchema,
  readSessionPackageResultSchema,
  releaseCapabilitySchema,
  restoreAttemptSchema,
  restoreRequestSchema,
  sessionMessageSchema,
  sessionMetaSchema,
  sessionPackageExportResultSchema,
  type SessionMigrationPort,
} from "../../../features/session-migration";

export function createSessionMigrationPort(): SessionMigrationPort {
  return {
    listSessions: async () => {
      const raw = await invoke<unknown>("list_sessions");
      return z.array(sessionMetaSchema).parse(raw);
    },

    getSessionMessages: async (providerId, sourcePath) => {
      const raw = await invoke<unknown>("get_session_messages", {
        providerId,
        sourcePath,
      });
      return z.array(sessionMessageSchema).parse(raw);
    },

    previewSessionMigration: async (providerId, sourcePath) => {
      const raw = await invoke<unknown>("preview_session_migration", {
        providerId,
        sourcePath,
      });
      return migratableSessionSchema.parse(raw);
    },

    exportSessionPackage: async (items, targetPath) => {
      const raw = await invoke<unknown>("export_session_package", {
        items,
        targetPath,
      });
      return sessionPackageExportResultSchema.parse(raw);
    },

    readSessionPackage: async (path) => {
      const raw = await invoke<unknown>("read_session_package", { path });
      return readSessionPackageResultSchema.parse(raw);
    },

    probeLocalProvider: async (providerId) => {
      const raw = await invoke<unknown>("probe_local_provider", { providerId });
      return localProviderProbeSchema.parse(raw);
    },

    getReleaseCapabilityMatrix: async () => {
      const raw = await invoke<unknown>("get_release_capability_matrix");
      return z.array(releaseCapabilitySchema).parse(raw);
    },

    restoreSessionPackage: async (request) => {
      const validatedRequest = restoreRequestSchema.parse(request);
      const raw = await invoke<unknown>("restore_session_package", {
        request: validatedRequest,
      });
      return z.array(restoreAttemptSchema).parse(raw);
    },

    verifyNativeReadback: async (attemptId) => {
      const raw = await invoke<unknown>("verify_native_readback", {
        attemptId,
      });
      return restoreAttemptSchema.parse(raw);
    },

    listRestoreAttempts: async () => {
      const raw = await invoke<unknown>("list_restore_attempts");
      return z.array(restoreAttemptSchema).parse(raw);
    },

    reconcileRestoreAttempts: async () => {
      const raw = await invoke<unknown>("reconcile_restore_attempts");
      return z.array(restoreAttemptSchema).parse(raw);
    },

    recordUserAttestation: async (attemptId, claimedStage, note) => {
      const raw = await invoke<unknown>("record_user_attestation", {
        attemptId,
        claimedStage,
        note,
      });
      return restoreAttemptSchema.parse(raw);
    },

    openRestoredSession: async (attemptId) => {
      const raw = await invoke<unknown>("open_restored_session", {
        attemptId,
      });
      return z.boolean().parse(raw);
    },

    pickDirectory: async (defaultPath) => {
      const raw = await invoke<unknown>("pick_directory", { defaultPath });
      return z.nullable(z.string()).parse(raw);
    },

    pickPackageFile: async () => {
      const raw = await invoke<unknown>("pick_session_package_file");
      return z.nullable(z.string()).parse(raw);
    },

    pickExportPath: async (defaultName: string) => {
      const raw = await invoke<unknown>("pick_session_package_export_path", {
        defaultName,
      });
      return z.nullable(z.string()).parse(raw);
    },
  };
}
