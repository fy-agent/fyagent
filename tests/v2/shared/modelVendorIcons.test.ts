import { describe, expect, it } from "vitest";

import {
  resolveModelVendorIcon,
  unknownModelVendorIconUrl,
} from "@/v2/shared/assets/models";

describe("resolveModelVendorIcon", () => {
  it("resolves local vendor assets from ownedBy then id prefixes, never a remote URL", () => {
    const openai = resolveModelVendorIcon("custom-router", "OpenAI");
    const claude = resolveModelVendorIcon("claude-sonnet-4");
    const unknown = resolveModelVendorIcon("mystery-router-9");

    expect(openai).toMatch(/\/src\/v2\/shared\/assets\/models\/openai\.svg$/);
    expect(claude).toMatch(/\/src\/v2\/shared\/assets\/models\/claude\.svg$/);
    expect(unknown).toBe(unknownModelVendorIconUrl);

    for (const url of [openai, claude, unknown, unknownModelVendorIconUrl]) {
      expect(url).toMatch(/^\/src\/v2\/shared\/assets\/models\//);
      expect(url).not.toMatch(/^https?:/i);
    }
  });
});
