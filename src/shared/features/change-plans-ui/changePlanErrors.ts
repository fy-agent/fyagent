import type { ChangePlanErrorCode } from "../change-plans";

export const CHANGE_PLAN_ERROR_CODES = new Set<ChangePlanErrorCode>([
  "unsupported_operation",
  "invalid_target",
  "target_not_found",
  "target_already_current",
  "secret_dependency_unavailable",
  "invalid_digest",
  "expired",
  "consumed",
  "stale",
  "plan_not_found",
  "job_not_found",
  "internal",
]);

export const JOB_REFRESH_INTERVAL_MS = 1000;

export function isActiveJobStatus(status: string): boolean {
  return status === "planned" || status === "running";
}

export function changePlanErrorCode(error: unknown): ChangePlanErrorCode {
  const candidate =
    typeof error === "string"
      ? error
      : typeof error === "object" && error !== null && "code" in error
        ? error.code
        : null;
  return typeof candidate === "string" &&
    CHANGE_PLAN_ERROR_CODES.has(candidate as ChangePlanErrorCode)
    ? (candidate as ChangePlanErrorCode)
    : "internal";
}
