import { invoke } from "@tauri-apps/api/core";

import type {
  RemotePushApplyResult,
  RemotePushPreviewResult,
  RemotePushRequest,
  SaveSshPushProfileRequest,
  SshConnectionTestResult,
  SshHostEntry,
  SshPushProfile,
  SshTargetSpec,
  WorkspacePackExportOptions,
  WorkspacePackExportOutcome,
  WorkspacePackExportPreview,
  WorkspacePackExportRequest,
  WorkspacePackImportOutcome,
  WorkspacePackImportPreview,
  WorkspacePackImportRequest,
  WorkspacePackInspectRequest,
  WorkspaceTransferPort,
  WslEnvironmentStatus,
  WslSyncRequest,
} from "../../../features/workspace-transfer";

export function createWorkspaceTransferPort(): WorkspaceTransferPort {
  return {
    pickExportPath: (defaultName: string) =>
      invoke<string | null>("pick_workspace_pack_export_path", {
        defaultName,
      }),
    pickImportFile: () => invoke<string | null>("pick_workspace_pack_file"),
    previewExport: (options: WorkspacePackExportOptions) =>
      invoke<WorkspacePackExportPreview>("preview_workspace_pack_export", {
        options,
      }),
    exportPack: (request: WorkspacePackExportRequest) =>
      invoke<WorkspacePackExportOutcome>("export_workspace_pack", { request }),
    inspectPack: (request: WorkspacePackInspectRequest) =>
      invoke<WorkspacePackImportPreview>("inspect_workspace_pack", { request }),
    applyImport: (request: WorkspacePackImportRequest) =>
      invoke<WorkspacePackImportOutcome>("apply_workspace_pack_import", {
        request,
      }),
    listSshHosts: () => invoke<SshHostEntry[]>("list_ssh_hosts"),
    testSshTarget: (target: SshTargetSpec) =>
      invoke<SshConnectionTestResult>("test_ssh_target", { target }),
    previewSshPush: (request: RemotePushRequest) =>
      invoke<RemotePushPreviewResult>("preview_ssh_config_push", { request }),
    applySshPush: (request: RemotePushRequest) =>
      invoke<RemotePushApplyResult>("apply_ssh_config_push", { request }),
    listSshPushProfiles: () =>
      invoke<SshPushProfile[]>("list_ssh_push_profiles"),
    saveSshPushProfile: (request: SaveSshPushProfileRequest) =>
      invoke<SshPushProfile>("save_ssh_push_profile", { request }),
    deleteSshPushProfile: (id: string) =>
      invoke<boolean>("delete_ssh_push_profile", { id }),
    listWslDistros: () => invoke<WslEnvironmentStatus>("list_wsl_distros"),
    previewWslSync: (request: WslSyncRequest) =>
      invoke<RemotePushPreviewResult>("preview_wsl_config_sync", { request }),
    applyWslSync: (request: WslSyncRequest) =>
      invoke<RemotePushApplyResult>("apply_wsl_config_sync", { request }),
  };
}
