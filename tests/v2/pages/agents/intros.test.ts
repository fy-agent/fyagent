import { describe, expect, it } from "vitest";

import { getAgentIntro } from "@/v2/pages/agents/intros";
import { AGENT_CATALOG_IDS } from "@/v2/shared/features/types";

describe("Agent directory product intros", () => {
  it("describes each product without naming FyAgent", () => {
    for (const id of AGENT_CATALOG_IDS) {
      const intro = getAgentIntro(id);
      if (intro === null) continue;
      expect(intro.paragraphs.length).toBeGreaterThan(0);
      for (const paragraph of intro.paragraphs) {
        expect(paragraph, id).not.toMatch(/FyAgent/iu);
      }
    }
  });

  it("keeps Qoder product copy free of Hooks", () => {
    const intro = getAgentIntro("qoderwork");
    expect(intro).not.toBeNull();
    for (const paragraph of intro!.paragraphs) {
      expect(paragraph).not.toMatch(/hooks/iu);
    }
  });
});
