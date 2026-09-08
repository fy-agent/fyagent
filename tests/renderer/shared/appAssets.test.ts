import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";

import { describe, expect, it } from "vitest";

import {
  getSupportedAppIcon,
  getSkillTargetIcon,
  skillTargetIconById,
  supportedAppIconById,
} from "@/shared/assets/apps";
import {
  MCP_TARGET_IDS,
  MCP_TARGETS,
  SKILL_TARGET_IDS,
  SKILL_TARGETS,
  SUPPORTED_APP_IDS,
  type SupportedAppId,
} from "@/shared/features/types";

const repositoryRoot = path.resolve(process.cwd());
const appAssetRoot = path.join(
  repositoryRoot,
  "src",
  "shared",
  "assets",
  "apps",
);
const agentAssetRoot = path.join(
  repositoryRoot,
  "src",
  "shared",
  "assets",
  "agents",
);

const copiedAssetNames = {
  opencode: "opencode.svg",
} as const;

const reviewedSources: Readonly<Record<keyof typeof copiedAssetNames, string>> =
  {
    opencode:
      "5eeb3a5f14dbbccc03111296ecfd3cd05cc98844150d4f9d77ba921847953380",
  };

const expectedAssetPaths: Readonly<Record<SupportedAppId, string>> = {
  qoderwork: path.join(agentAssetRoot, "qoderwork.png"),
  "trae-work": path.join(agentAssetRoot, "trae-work.png"),
  workbuddy: path.join(agentAssetRoot, "workbuddy.png"),
  grokbuild: path.join(appAssetRoot, "grokbuild.svg"),
  codex: path.join(agentAssetRoot, "codex.svg"),
  claude: path.join(agentAssetRoot, "claude-code.svg"),
  opencode: path.join(appAssetRoot, copiedAssetNames.opencode),
};

describe("Supported application assets", () => {
  it("maps every exact supported application ID to one bundled local asset", () => {
    expect(Object.keys(supportedAppIconById)).toEqual(SUPPORTED_APP_IDS);

    for (const id of SUPPORTED_APP_IDS) {
      expect(getSupportedAppIcon(id)).toBe(supportedAppIconById[id]);
      expect(getSupportedAppIcon(id)).toMatch(
        /^\/src\/shared\/assets\/(?:agents|apps)\//,
      );
    }
  });

  it("keeps Skills and MCP on the same seven catalog-aligned targets", () => {
    expect(Object.keys(skillTargetIconById)).toEqual(SKILL_TARGET_IDS);
    expect(Object.keys(supportedAppIconById)).toEqual(SUPPORTED_APP_IDS);
    expect(SKILL_TARGET_IDS).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude",
      "opencode",
    ]);
    expect(SKILL_TARGET_IDS).toBe(MCP_TARGET_IDS);
    expect(SKILL_TARGETS).toBe(MCP_TARGETS);
    expect(SUPPORTED_APP_IDS).toEqual(SKILL_TARGET_IDS);
    expect(getSkillTargetIcon("qoderwork")).toMatch(
      /\/src\/shared\/assets\/agents\/qoderwork\.png$/,
    );
    expect(getSkillTargetIcon("trae-work")).toMatch(
      /\/src\/shared\/assets\/agents\/trae-work\.png$/,
    );
    expect(getSkillTargetIcon("workbuddy")).toMatch(
      /\/src\/shared\/assets\/agents\/workbuddy\.png$/,
    );
  });

  it("keeps non-Agent app art byte-identical to reviewed assets", () => {
    for (const id of Object.keys(reviewedSources) as Array<
      keyof typeof reviewedSources
    >) {
      expect(
        createHash("sha256")
          .update(readFileSync(path.join(appAssetRoot, copiedAssetNames[id])))
          .digest("hex"),
      ).toBe(reviewedSources[id]);
    }
  });

  it("keeps every supported application asset resolvable and non-empty", () => {
    for (const id of SUPPORTED_APP_IDS) {
      expect(readFileSync(expectedAssetPaths[id]).byteLength).toBeGreaterThan(
        0,
      );
    }
  });
});
