export const XAI_BIND_ERROR_CODES = [
  "invalid_request",
  "account_unavailable",
  "provider_conflict",
  "apply_failed_rolled_back",
  "rollback_partial_state_unknown",
] as const;

export type XaiBindErrorCode = (typeof XAI_BIND_ERROR_CODES)[number];

export function xaiBindErrorCode(value: unknown): XaiBindErrorCode | null {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return null;
  const keys = Object.keys(value);
  if (keys.length !== 1 || keys[0] !== "code" || !("code" in value))
    return null;
  return XAI_BIND_ERROR_CODES.find((code) => code === value.code) ?? null;
}

export function isXaiSubscriptionModelId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/u.test(value)
  );
}
