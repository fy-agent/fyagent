import grokBuildIconUrl from "../apps/grokbuild.svg";
import claudeCodeIconUrl from "./claude-code.svg";
import codexIconUrl from "./codex.svg";
import openCodeIconUrl from "./opencode.svg";
import qoderWorkIconUrl from "./qoderwork.png";
import traeWorkIconUrl from "./trae-work.png";
import workBuddyIconUrl from "./workbuddy.png";

export const agentIconIds = [
  "qoderwork",
  "trae-work",
  "workbuddy",
  "grokbuild",
  "codex",
  "claude-code",
  "opencode",
] as const;

export type AgentIconId = (typeof agentIconIds)[number];

export type AgentBrandBackground = "transparent" | "surface";
export type AgentBrandCorner = "none" | "soft" | "rounded";

export interface AgentBrandOptics {
  readonly opticalScale: number;
  readonly background: AgentBrandBackground;
  readonly corner: AgentBrandCorner;
}

export interface AgentBrandAsset {
  readonly iconUrl: string;
  readonly list: AgentBrandOptics;
  readonly detail: AgentBrandOptics;
}

export const agentBrandById = {
  qoderwork: {
    iconUrl: qoderWorkIconUrl,
    list: {
      opticalScale: 1,
      background: "transparent",
      corner: "soft",
    },
    detail: {
      opticalScale: 1,
      background: "transparent",
      corner: "soft",
    },
  },
  "trae-work": {
    iconUrl: traeWorkIconUrl,
    list: {
      opticalScale: 1,
      background: "transparent",
      corner: "soft",
    },
    detail: {
      opticalScale: 1,
      background: "transparent",
      corner: "soft",
    },
  },
  workbuddy: {
    iconUrl: workBuddyIconUrl,
    list: {
      opticalScale: 0.94,
      background: "transparent",
      corner: "rounded",
    },
    detail: {
      opticalScale: 0.94,
      background: "transparent",
      corner: "rounded",
    },
  },
  grokbuild: {
    iconUrl: grokBuildIconUrl,
    list: {
      opticalScale: 0.92,
      background: "transparent",
      corner: "none",
    },
    detail: {
      opticalScale: 0.92,
      background: "transparent",
      corner: "none",
    },
  },
  codex: {
    iconUrl: codexIconUrl,
    list: {
      opticalScale: 0.9,
      background: "transparent",
      corner: "none",
    },
    detail: {
      opticalScale: 0.9,
      background: "transparent",
      corner: "none",
    },
  },
  "claude-code": {
    iconUrl: claudeCodeIconUrl,
    list: {
      opticalScale: 0.92,
      background: "transparent",
      corner: "none",
    },
    detail: {
      opticalScale: 0.92,
      background: "transparent",
      corner: "none",
    },
  },
  opencode: {
    iconUrl: openCodeIconUrl,
    list: {
      opticalScale: 0.9,
      background: "transparent",
      corner: "none",
    },
    detail: {
      opticalScale: 0.9,
      background: "transparent",
      corner: "none",
    },
  },
} as const satisfies Readonly<Record<AgentIconId, AgentBrandAsset>>;

export const agentIconById = Object.fromEntries(
  agentIconIds.map((id) => [id, agentBrandById[id].iconUrl]),
) as Readonly<Record<AgentIconId, string>>;

export function getAgentBrand(id: AgentIconId): AgentBrandAsset {
  return agentBrandById[id];
}

export function getAgentIcon(id: AgentIconId): string {
  return agentIconById[id];
}
