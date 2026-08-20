import type {
  InstallerErrorDto,
  JobSnapshot,
  LocalInstallStatus,
} from "./types";

export function asInstallerError(error: unknown): InstallerErrorDto | null {
  if (!error || typeof error !== "object") return null;
  const candidate = error as Partial<InstallerErrorDto>;
  return typeof candidate.code === "string" &&
    typeof candidate.messageKey === "string" &&
    candidate.details
    ? (candidate as InstallerErrorDto)
    : null;
}

export function latestKnownInstallerError(
  local: LocalInstallStatus | undefined,
  job: JobSnapshot | null | undefined,
  errors: readonly unknown[],
): InstallerErrorDto | null {
  if (job?.error) return job.error;
  if (local?.state === "ambiguous") return local.error;

  for (const error of errors) {
    const installerError = asInstallerError(error);
    if (installerError) return installerError;
  }

  return null;
}

export function installerErrorDetailsForCopy(
  error: InstallerErrorDto | null,
): string | null {
  if (!error) return null;
  return JSON.stringify(error, null, 2);
}
