import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

import {
  agentBrandById,
  agentIconById,
  agentIconIds,
  getAgentBrand,
  getAgentIcon,
} from "@/shared/assets/agents";

const repositoryRoot = path.resolve(process.cwd());
const assetRoot = path.join(
  repositoryRoot,
  "src",
  "shared",
  "assets",
  "agents",
);

const officialAssetDigests = {
  "qoderwork.png":
    "7f9afcd051e4a4de4743a29a426d5a50b2f5f7189fdf658e876f9330c775f178",
  "trae-work.png":
    "49d523938a22af5a70dd79923725df38674823026e2f917e76337319969f4af4",
} as const;

function assetPath(fileName: string): string {
  return path.join(assetRoot, fileName);
}

function sha256(fileName: string): string {
  return createHash("sha256")
    .update(readFileSync(assetPath(fileName)))
    .digest("hex");
}

function readPngMetadata(fileName: string): {
  width: number;
  height: number;
  bitDepth: number;
  colorType: number;
} {
  const bytes = readFileSync(assetPath(fileName));
  expect(bytes.subarray(0, 8).toString("hex")).toBe("89504e470d0a1a0a");
  expect(bytes.subarray(12, 16).toString("ascii")).toBe("IHDR");

  return {
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
    bitDepth: bytes[24],
    colorType: bytes[25],
  };
}

describe("Agent catalog assets", () => {
  it("maps every exact native catalog ID to one bundled local asset", () => {
    expect(agentIconIds).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude-code",
      "opencode",
    ]);
    expect(Object.keys(agentIconById)).toEqual(agentIconIds);
    expect(Object.keys(agentBrandById)).toEqual(agentIconIds);

    for (const id of agentIconIds) {
      expect(getAgentIcon(id)).toBe(agentIconById[id]);
      expect(getAgentIcon(id)).toMatch(
        /^\/src\/shared\/assets\/(?:agents|apps)\//,
      );
      expect(getAgentBrand(id)).toBe(agentBrandById[id]);
      expect(getAgentBrand(id).iconUrl).toBe(getAgentIcon(id));
      for (const size of ["list", "detail"] as const) {
        const optics = getAgentBrand(id)[size];
        expect(optics.opticalScale).toBeGreaterThan(0);
        expect(optics.opticalScale).toBeLessThanOrEqual(1);
        expect(["transparent", "surface"]).toContain(optics.background);
        expect(["none", "soft", "rounded"]).toContain(optics.corner);
      }
    }
  });

  it("preserves the exact official QoderWork and TRAE Work source bytes", () => {
    for (const [fileName, digest] of Object.entries(officialAssetDigests)) {
      expect(sha256(fileName)).toBe(digest);
    }

    expect(readPngMetadata("qoderwork.png")).toEqual({
      width: 256,
      height: 256,
      bitDepth: 8,
      colorType: 6,
    });
    expect(readPngMetadata("trae-work.png")).toEqual({
      width: 48,
      height: 48,
      bitDepth: 8,
      colorType: 6,
    });
  });

  it("keeps reviewed WorkBuddy, Codex, and Claude Code art byte-identical", () => {
    // Digests come from the immutable pre-consolidation assets, not another
    // duplicate tree that would need to be maintained just for comparison.
    const reviewedSources = {
      "workbuddy.png":
        "060e5e0fe1fce063e24b809a2d655df5a32ef36d97a7322e33b22c245570b868",
      "codex.svg":
        "f3cf560e84bd85915e32b887cb2abd323529be62d8b4001d2f778669b77eda35",
      "claude-code.svg":
        "a3101f3047a119aa11825ad9369510f0c472428c8c52d420e31bc62db44a8364",
    } as const;

    for (const [fileName, digest] of Object.entries(reviewedSources))
      expect(sha256(fileName)).toBe(digest);

    expect(readPngMetadata("workbuddy.png")).toMatchObject({
      width: 512,
      height: 512,
      bitDepth: 8,
      colorType: 6,
    });
  });
});
