import { describe, expect, it } from "vitest";

import {
  isProviderRestoreSupported,
  type LocalProviderProbe,
} from "@/shared/features/session-migration";

const geminiWriteOnlyProbe: LocalProviderProbe = {
  providerId: "gemini",
  installed: true,
  detectedVersion: "0.46.0",
  extractionSupported: false,
  writeSupported: true,
  reasonCode: "extractionRuleUnavailable",
};

describe("independent session restore capability gate", () => {
  it("allows a verified native writer when only extraction is unavailable", () => {
    expect(isProviderRestoreSupported(geminiWriteOnlyProbe)).toEqual({
      supported: true,
    });
  });

  it("still blocks an installed Codex before native initialization", () => {
    const result = isProviderRestoreSupported({
      ...geminiWriteOnlyProbe,
      providerId: "codex",
      detectedVersion: "0.154.0",
      extractionSupported: true,
      writeSupported: false,
      reasonCode: "targetStoreUnidentified",
    });
    expect(result.supported).toBe(false);
    expect(result.reason).toContain("首次安装 Codex");
  });

  it("still blocks an unverified writer version", () => {
    const result = isProviderRestoreSupported({
      ...geminiWriteOnlyProbe,
      writeSupported: false,
      reasonCode: "providerVersionUnsupported",
    });
    expect(result.supported).toBe(false);
    expect(result.reason).toContain("版本尚未支持");
  });

  it("still blocks an absent provider and missing probe", () => {
    expect(
      isProviderRestoreSupported({
        ...geminiWriteOnlyProbe,
        installed: false,
        writeSupported: false,
        reasonCode: "providerNotInstalled",
      }).supported,
    ).toBe(false);
    expect(isProviderRestoreSupported().supported).toBe(false);
  });
});
