import type {
  CpuArchitecture,
  DesktopPlatform,
  InstallResult,
  InstalledApplication,
  InstalledApplicationSummary,
  InstallerDiagnosticDetails,
  InstallerErrorCode,
  InstallerErrorDto,
  InstallerWarningCode,
  JobProgress,
  JobSnapshot,
  JobStage,
  LocalInstallStatus,
  PlatformVersion,
  ProgressPhase,
  RemoteReleaseStatus,
  SuggestedAction,
  UnsupportedReason,
} from "./types";

export const CODEX_DESKTOP_PAYLOAD_ERROR =
  "Codex desktop installer response is invalid";

const desktopPlatforms: Readonly<Record<DesktopPlatform, true>> = {
  windows: true,
  macos: true,
};

const cpuArchitectures: Readonly<Record<CpuArchitecture, true>> = {
  x86_64: true,
  aarch64: true,
  x86_64_unsupported_mac: true,
  unsupported: true,
};

const unsupportedReasons: Readonly<Record<UnsupportedReason, true>> = {
  platform: true,
  architecture: true,
  os_version: true,
};

const jobStages: Readonly<Record<JobStage, true>> = {
  checking: true,
  preflight: true,
  downloading: true,
  installing: true,
  verifying_installation: true,
  succeeded: true,
  failed: true,
  cancelled: true,
};

const progressPhases: Readonly<Record<ProgressPhase, true>> = {
  download: true,
  verification: true,
  installation: true,
};

const installerWarningCodes: Readonly<Record<InstallerWarningCode, true>> = {
  temp_cleanup_failed: true,
  mac_dmg_detach_warning: true,
  log_write_failed: true,
  event_emit_failed: true,
  remote_check_failed_local_available: true,
};

const installerErrorCodes: Readonly<Record<InstallerErrorCode, true>> = {
  PLATFORM_UNSUPPORTED: true,
  OS_VERSION_UNSUPPORTED: true,
  ARCHITECTURE_UNSUPPORTED: true,
  SOURCE_UNAVAILABLE: true,
  RELEASE_METADATA_INVALID: true,
  RELEASE_NOT_AVAILABLE: true,
  METADATA_CHANGED: true,
  REDIRECT_REJECTED: true,
  DOWNLOAD_FAILED: true,
  DOWNLOAD_TIMEOUT: true,
  DOWNLOAD_CANCELLED: true,
  INSUFFICIENT_DISK_SPACE: true,
  CHECKSUM_MISMATCH: true,
  PACKAGE_PARSE_FAILED: true,
  PACKAGE_IDENTITY_MISMATCH: true,
  PACKAGE_ARCHITECTURE_MISMATCH: true,
  PACKAGE_SIGNATURE_INVALID: true,
  WINDOWS_PACKAGE_IN_USE: true,
  WINDOWS_DEPLOYMENT_BLOCKED: true,
  WINDOWS_DEPENDENCY_MISSING: true,
  WINDOWS_DEPLOYMENT_FAILED: true,
  MULTIPLE_INSTALLATIONS: true,
  MAC_DMG_MOUNT_FAILED: true,
  MAC_APP_NOT_FOUND: true,
  MAC_BUNDLE_ID_MISMATCH: true,
  MAC_APP_RUNNING: true,
  MAC_MULTIPLE_INSTALLATIONS: true,
  MAC_TARGET_PATH_CONFLICT: true,
  MAC_COPY_FAILED: true,
  MAC_DMG_DETACH_FAILED: true,
  INSTALLATION_VERIFY_FAILED: true,
  LAUNCH_FAILED: true,
  JOB_ALREADY_RUNNING: true,
  JOB_NOT_FOUND: true,
  INTERNAL_ERROR: true,
};

const suggestedActions: Readonly<Record<SuggestedAction, true>> = {
  retry: true,
  refresh: true,
  close_target_app_and_retry: true,
  contact_administrator: true,
  free_disk_space: true,
  resolve_path_conflict: true,
  open_logs: true,
  none: true,
};

const releaseIdPattern = /^v1:[0-9a-f]{64}$/;
const rfc3339Pattern =
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;

function invalidPayload(): never {
  throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
}

function exactRecord(
  value: unknown,
  expectedKeys: readonly string[],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return invalidPayload();
  const record = value as Record<string, unknown>;
  const actualKeys = Object.keys(record).sort();
  const sortedExpected = [...expectedKeys].sort();
  if (
    actualKeys.length !== sortedExpected.length ||
    actualKeys.some((key, index) => key !== sortedExpected[index])
  )
    return invalidPayload();
  return record;
}

function knownString<T extends string>(
  value: unknown,
  values: Readonly<Record<T, true>>,
): T {
  if (
    typeof value !== "string" ||
    !Object.prototype.hasOwnProperty.call(values, value)
  )
    return invalidPayload();
  return value as T;
}

function string(value: unknown, requireNonempty = false): string {
  if (
    typeof value !== "string" ||
    (requireNonempty && value.trim().length === 0)
  )
    return invalidPayload();
  return value;
}

function nullableString(value: unknown): string | null {
  return value === null ? null : string(value);
}

function boolean(value: unknown): boolean {
  if (typeof value !== "boolean") return invalidPayload();
  return value;
}

function safeInteger(
  value: unknown,
  minimum = 0,
  maximum = Number.MAX_SAFE_INTEGER,
) {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < minimum ||
    value > maximum
  )
    return invalidPayload();
  return value;
}

function nullableSafeInteger(value: unknown): number | null {
  return value === null ? null : safeInteger(value);
}

function percentage(value: unknown): number | null {
  if (value === null) return null;
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < 0 ||
    value > 100
  )
    return invalidPayload();
  return value;
}

function timestamp(value: unknown): string {
  const result = string(value, true);
  if (!rfc3339Pattern.test(result) || !Number.isFinite(Date.parse(result)))
    return invalidPayload();
  return result;
}

function releaseId(value: unknown): string {
  const result = string(value, true);
  if (!releaseIdPattern.test(result)) return invalidPayload();
  return result;
}

function parsePlatformVersion(value: unknown): PlatformVersion {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return invalidPayload();
  const kind = (value as Record<string, unknown>).kind;
  if (kind === "windows_msix") {
    const record = exactRecord(value, [
      "kind",
      "major",
      "minor",
      "build",
      "revision",
    ]);
    return {
      kind,
      major: safeInteger(record.major, 0, 65_535),
      minor: safeInteger(record.minor, 0, 65_535),
      build: safeInteger(record.build, 0, 65_535),
      revision: safeInteger(record.revision, 0, 65_535),
    };
  }
  if (kind === "mac_bundle") {
    const record = exactRecord(value, ["kind", "bundleVersion"]);
    return { kind, bundleVersion: string(record.bundleVersion, true) };
  }
  return invalidPayload();
}

function parseInstalledApplication(value: unknown): InstalledApplication {
  const record = exactRecord(value, [
    "stableIdentity",
    "displayName",
    "displayVersion",
    "platformVersion",
    "architecture",
  ]);
  return {
    stableIdentity: string(record.stableIdentity, true),
    displayName: nullableString(record.displayName),
    displayVersion: nullableString(record.displayVersion),
    platformVersion: parsePlatformVersion(record.platformVersion),
    architecture: knownString(record.architecture, cpuArchitectures),
  };
}

function parseInstalledApplicationSummary(
  value: unknown,
): InstalledApplicationSummary {
  const record = exactRecord(value, [
    "stableIdentity",
    "displayVersion",
    "platformVersion",
    "architecture",
  ]);
  return {
    stableIdentity: string(record.stableIdentity, true),
    displayVersion: nullableString(record.displayVersion),
    platformVersion: parsePlatformVersion(record.platformVersion),
    architecture: knownString(record.architecture, cpuArchitectures),
  };
}

function parseInstallerDetails(value: unknown): InstallerDiagnosticDetails {
  const record = exactRecord(value, [
    "endpointKind",
    "attempt",
    "maxAttempts",
    "httpStatus",
    "platformErrorCode",
    "redactedMessage",
    "context",
  ]);
  if (
    typeof record.context !== "object" ||
    record.context === null ||
    Array.isArray(record.context)
  )
    return invalidPayload();
  const context: Record<string, string> = {};
  for (const [key, value] of Object.entries(record.context)) {
    context[key] = string(value);
  }
  return {
    endpointKind: nullableString(record.endpointKind),
    attempt: nullableSafeInteger(record.attempt),
    maxAttempts: nullableSafeInteger(record.maxAttempts),
    httpStatus:
      record.httpStatus === null
        ? null
        : safeInteger(record.httpStatus, 0, 65_535),
    platformErrorCode: nullableString(record.platformErrorCode),
    redactedMessage: nullableString(record.redactedMessage),
    context,
  };
}

function parseInstallerError(value: unknown): InstallerErrorDto {
  const record = exactRecord(value, [
    "code",
    "stage",
    "messageKey",
    "retryable",
    "suggestedAction",
    "details",
  ]);
  return {
    code: knownString(record.code, installerErrorCodes),
    stage: record.stage === null ? null : knownString(record.stage, jobStages),
    messageKey: string(record.messageKey, true),
    retryable: boolean(record.retryable),
    suggestedAction: knownString(record.suggestedAction, suggestedActions),
    details: parseInstallerDetails(record.details),
  };
}

export function parseLocalInstallStatus(value: unknown): LocalInstallStatus {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return invalidPayload();
  const state = (value as Record<string, unknown>).state;
  switch (state) {
    case "not_installed": {
      const record = exactRecord(value, ["state", "platform", "architecture"]);
      return {
        state,
        platform: knownString(record.platform, desktopPlatforms),
        architecture: knownString(record.architecture, cpuArchitectures),
      };
    }
    case "installed": {
      const record = exactRecord(value, ["state", "application"]);
      return {
        state,
        application: parseInstalledApplication(record.application),
      };
    }
    case "unsupported": {
      const record = exactRecord(value, ["state", "reason"]);
      return {
        state,
        reason: knownString(record.reason, unsupportedReasons),
      };
    }
    case "ambiguous": {
      const record = exactRecord(value, ["state", "candidates", "error"]);
      if (!Array.isArray(record.candidates)) return invalidPayload();
      return {
        state,
        candidates: record.candidates.map(parseInstalledApplicationSummary),
        error: parseInstallerError(record.error),
      };
    }
    default:
      return invalidPayload();
  }
}

export function parseRemoteReleaseStatus(value: unknown): RemoteReleaseStatus {
  const record = exactRecord(value, [
    "releaseId",
    "displayVersion",
    "platformVersion",
    "downloadSizeHint",
    "checkedAt",
  ]);
  return {
    releaseId: releaseId(record.releaseId),
    displayVersion: string(record.displayVersion, true),
    platformVersion: parsePlatformVersion(record.platformVersion),
    downloadSizeHint:
      record.downloadSizeHint === null
        ? null
        : safeInteger(record.downloadSizeHint, 1),
    checkedAt: timestamp(record.checkedAt),
  };
}

function parseJobProgress(value: unknown): JobProgress {
  const record = exactRecord(value, [
    "phase",
    "completedBytes",
    "totalBytes",
    "percent",
  ]);
  const completedBytes = nullableSafeInteger(record.completedBytes);
  const totalBytes = nullableSafeInteger(record.totalBytes);
  return {
    phase: knownString(record.phase, progressPhases),
    completedBytes,
    totalBytes,
    percent: percentage(record.percent),
  };
}

function parseInstallResult(value: unknown): InstallResult {
  const record = exactRecord(value, ["installed", "warnings"]);
  if (!Array.isArray(record.warnings)) return invalidPayload();
  return {
    installed: parseInstalledApplicationSummary(record.installed),
    warnings: record.warnings.map((warning) =>
      knownString(warning, installerWarningCodes),
    ),
  };
}

export function parseJobSnapshot(value: unknown): JobSnapshot {
  const record = exactRecord(value, [
    "jobId",
    "sequence",
    "stage",
    "release",
    "startedAt",
    "updatedAt",
    "progress",
    "cancellable",
    "result",
    "error",
  ]);
  const stage = knownString(record.stage, jobStages);
  const startedAt = timestamp(record.startedAt);
  const updatedAt = timestamp(record.updatedAt);
  if (Date.parse(updatedAt) < Date.parse(startedAt)) return invalidPayload();
  const result =
    record.result === null ? null : parseInstallResult(record.result);
  const error =
    record.error === null ? null : parseInstallerError(record.error);
  if (
    (stage === "succeeded" && (result === null || error !== null)) ||
    (stage === "failed" && (result !== null || error === null)) ||
    (stage !== "succeeded" && stage !== "failed" && (result || error))
  )
    return invalidPayload();
  return {
    jobId: string(record.jobId, true),
    sequence: safeInteger(record.sequence),
    stage,
    release: parseRemoteReleaseStatus(record.release),
    startedAt,
    updatedAt,
    progress:
      record.progress === null ? null : parseJobProgress(record.progress),
    cancellable: boolean(record.cancellable),
    result,
    error,
  };
}

export function parseOptionalJobSnapshot(value: unknown): JobSnapshot | null {
  return value === null ? null : parseJobSnapshot(value);
}

export function assertExpectedReleaseId(value: string): string {
  return releaseId(value);
}
