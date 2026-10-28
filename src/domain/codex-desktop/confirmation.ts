import {
  assertExpectedReleaseId,
  CODEX_DESKTOP_PAYLOAD_ERROR,
} from "./parsers";
import { isDownloadSourceUrl } from "../installation-source";

export interface CodexInstallPreflight {
  contractVersion: 1;
  confirmationId: string;
  expectedReleaseId: string;
  platform: "macos" | "windows";
  architecture: "aarch64" | "x86_64";
  displayVersion: string;
  downloadUrl: string;
  targetLabel: string;
  stagingLabel: string;
  customStaging: boolean;
  updating: boolean;
  availableBytes: number;
  downloadSizeHint: number | null;
}

export interface CodexStagingSelection {
  contractVersion: 1;
  capabilityId: string;
  label: string;
  availableBytes: number;
}
function isBoundedLabel(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 512 &&
    !Array.from(value).some(
      (char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127,
    )
  );
}

function isPositiveSafeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

export function assertInstallConfirmationId(value: string): string {
  if (!/^i1:[a-f0-9]{32}$/.test(value))
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  return value;
}

export function parseCodexInstallPreflight(
  value: unknown,
  expectedReleaseId: string,
): CodexInstallPreflight {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  const record = value as Record<string, unknown>;
  const keys = [
    "contractVersion",
    "confirmationId",
    "expectedReleaseId",
    "platform",
    "architecture",
    "displayVersion",
    "downloadUrl",
    "targetLabel",
    "stagingLabel",
    "customStaging",
    "updating",
    "availableBytes",
    "downloadSizeHint",
  ];
  if (
    Object.keys(record).length !== keys.length ||
    !keys.every((key) => key in record) ||
    record.contractVersion !== 1 ||
    typeof record.confirmationId !== "string" ||
    record.expectedReleaseId !== assertExpectedReleaseId(expectedReleaseId) ||
    !["macos", "windows"].includes(String(record.platform)) ||
    !["aarch64", "x86_64"].includes(String(record.architecture)) ||
    !isBoundedLabel(record.displayVersion) ||
    !isDownloadSourceUrl(record.downloadUrl) ||
    !isBoundedLabel(record.targetLabel) ||
    !isBoundedLabel(record.stagingLabel) ||
    typeof record.customStaging !== "boolean" ||
    typeof record.updating !== "boolean" ||
    !isPositiveSafeInteger(record.availableBytes) ||
    (record.downloadSizeHint !== null &&
      !isPositiveSafeInteger(record.downloadSizeHint))
  ) {
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  }
  assertInstallConfirmationId(record.confirmationId);
  return record as unknown as CodexInstallPreflight;
}

export function assertStagingCapabilityId(value: string): string {
  if (!/^s1:[a-f0-9]{32}$/.test(value))
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  return value;
}

export function parseCodexStagingSelection(
  value: unknown,
): CodexStagingSelection | null {
  if (value === null) return null;
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  const record = value as Record<string, unknown>;
  const keys = ["contractVersion", "capabilityId", "label", "availableBytes"];
  if (
    Object.keys(record).length !== keys.length ||
    !keys.every((key) => key in record) ||
    record.contractVersion !== 1 ||
    typeof record.capabilityId !== "string" ||
    !isBoundedLabel(record.label) ||
    !isPositiveSafeInteger(record.availableBytes)
  ) {
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  }
  assertStagingCapabilityId(record.capabilityId);
  return record as unknown as CodexStagingSelection;
}
