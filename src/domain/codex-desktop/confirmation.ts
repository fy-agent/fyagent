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
  updating: boolean;
  availableBytes: number;
  downloadSizeHint: number | null;
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
    "updating",
    "availableBytes",
    "downloadSizeHint",
  ];
  const label = (value: unknown) =>
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 512 &&
    !Array.from(value).some(
      (char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127,
    );
  const size = (value: unknown) =>
    typeof value === "number" && Number.isSafeInteger(value) && value > 0;
  if (
    Object.keys(record).length !== keys.length ||
    !keys.every((key) => key in record) ||
    record.contractVersion !== 1 ||
    typeof record.confirmationId !== "string" ||
    record.expectedReleaseId !== assertExpectedReleaseId(expectedReleaseId) ||
    !["macos", "windows"].includes(String(record.platform)) ||
    !["aarch64", "x86_64"].includes(String(record.architecture)) ||
    !label(record.displayVersion) ||
    !isDownloadSourceUrl(record.downloadUrl) ||
    !label(record.targetLabel) ||
    typeof record.updating !== "boolean" ||
    !size(record.availableBytes) ||
    (record.downloadSizeHint !== null && !size(record.downloadSizeHint))
  ) {
    throw new Error(CODEX_DESKTOP_PAYLOAD_ERROR);
  }
  assertInstallConfirmationId(record.confirmationId);
  return record as unknown as CodexInstallPreflight;
}
