import { describe, expect, it } from "vitest";

import { claudeDesktopProviderPresets } from "@/domain/configuration/presets/claudeDesktopProviderPresets";
import { providerPresets as claudeProviderPresets } from "@/domain/configuration/presets/claudeProviderPresets";
import { codexProviderPresets } from "@/domain/configuration/presets/codexProviderPresets";
import { geminiProviderPresets } from "@/domain/configuration/presets/geminiProviderPresets";
import {
  grokBuildOfficialPreset,
  grokBuildProviderPresets,
} from "@/domain/configuration/presets/grokBuildProviderPresets";
import { hermesProviderPresets } from "@/domain/configuration/presets/hermesProviderPresets";
import { openclawProviderPresets } from "@/domain/configuration/presets/openclawProviderPresets";
import { opencodeProviderPresets } from "@/domain/configuration/presets/opencodeProviderPresets";

const presetGroups: Array<[string, readonly unknown[]]> = [
  ["claudeDesktop", claudeDesktopProviderPresets],
  ["claude", claudeProviderPresets],
  ["codex", codexProviderPresets],
  ["gemini", geminiProviderPresets],
  ["grokBuild", [grokBuildOfficialPreset, ...grokBuildProviderPresets]],
  ["hermes", hermesProviderPresets],
  ["openclaw", openclawProviderPresets],
  ["opencode", opencodeProviderPresets],
];

const partnerMetadataKeys = [
  "isPartner",
  "primePartner",
  "partnerPromotionKey",
] as const;
const trackingQueryKeys = new Set(["affiliate", "ic", "ref", "referral"]);

describe("provider preset promotion boundary", () => {
  it("keeps partner metadata and tracking parameters out of every preset", () => {
    for (const [app, presets] of presetGroups) {
      for (const preset of presets) {
        const record = preset as Record<string, unknown>;
        const label = `${app}:${String(record.id ?? record.name ?? "unknown")}`;

        for (const key of partnerMetadataKeys) {
          expect(record, `${label} contains ${key}`).not.toHaveProperty(key);
        }

        for (const field of ["websiteUrl", "apiKeyUrl"] as const) {
          const value = record[field];
          if (typeof value !== "string" || value.trim() === "") continue;

          const url = new URL(value);
          const trackingKeys = [...url.searchParams.keys()].filter((key) => {
            const normalized = key.toLowerCase();
            return (
              trackingQueryKeys.has(normalized) || normalized.startsWith("utm_")
            );
          });
          expect(
            trackingKeys,
            `${label}.${field} contains tracking query data`,
          ).toEqual([]);
        }
      }
    }
  });
});
