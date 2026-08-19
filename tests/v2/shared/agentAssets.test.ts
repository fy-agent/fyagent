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
} from "@/v2/shared/assets/agents";

const repositoryRoot = path.resolve(process.cwd());
const assetRoot = path.join(
  repositoryRoot,
  "src",
  "v2",
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

describe("V2 Agent catalog assets", () => {
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
        /^\/src\/v2\/shared\/assets\/(?:agents|apps)\//,
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

  it("keeps reviewed WorkBuddy, Codex, and Claude Code art as V2-owned copies", () => {
    const reviewedSources = {
      "workbuddy.png": path.join(
        repositoryRoot,
        "src",
        "assets",
        "workbuddy-icon-512.png",
      ),
      "codex.svg": path.join(
        repositoryRoot,
        "src",
        "icons",
        "extracted",
        "openai.svg",
      ),
      "claude-code.svg": path.join(
        repositoryRoot,
        "src",
        "icons",
        "extracted",
        "claude.svg",
      ),
    } as const;

    for (const [fileName, sourcePath] of Object.entries(reviewedSources)) {
      expect(readFileSync(assetPath(fileName))).toEqual(
        readFileSync(sourcePath),
      );
    }

    expect(readPngMetadata("workbuddy.png")).toMatchObject({
      width: 512,
      height: 512,
      bitDepth: 8,
      colorType: 6,
    });
  });
});
