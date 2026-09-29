import { describe, expect, it } from "vitest";

import {
  grokLatestLabel,
  grokOwnerCopy,
  parseGrokToolSnapshot,
} from "@/shared/features/grok-tooling";

describe("parseGrokToolSnapshot", () => {
  it("reads native and npm owners without mixing latest labels", () => {
    expect(
      parseGrokToolSnapshot([
        {
          name: "grok",
          version: "1.0.5",
          latest_version: "1.0.6",
          error: null,
          installed_but_broken: false,
          distribution_owner: "native_internal",
          latest_source: "native_internal",
        },
      ]),
    ).toEqual({
      localVersion: "1.0.5",
      latestVersion: "1.0.6",
      distributionOwner: "native_internal",
      latestSource: "native_internal",
      latestAuthority: null,
      installedButBroken: false,
      error: null,
    });
    expect(grokOwnerCopy("native_internal")).toBe("官方命令行");
    expect(grokLatestLabel("official_npm")).toBe("官方 npm 最新");
    expect(grokLatestLabel("native_internal")).toBe("官方命令行最新");
  });

  it("labels a mirror-fallback latest as unconfirmed official latest", () => {
    const snapshot = parseGrokToolSnapshot([
      {
        name: "grok",
        version: null,
        latest_version: "1.0.44",
        error: null,
        installed_but_broken: false,
        latest_source: "official_npm",
        latest_authority: "mirror_fallback",
      },
    ]);
    expect(snapshot.latestAuthority).toBe("mirror_fallback");
    expect(
      grokLatestLabel(snapshot.latestSource, snapshot.latestAuthority),
    ).toBe("镜像版本（官方最新未确认）");
    expect(grokLatestLabel("official_npm", "official")).toBe("官方 npm 最新");
    expect(
      parseGrokToolSnapshot([
        {
          name: "grok",
          version: null,
          latest_version: "1.0.44",
          error: null,
          installed_but_broken: false,
          latest_authority: "official",
        },
      ]).latestAuthority,
    ).toBe("official");
  });

  it("rejects unknown latest authority values", () => {
    expect(() =>
      parseGrokToolSnapshot([
        {
          name: "grok",
          version: null,
          latest_version: "1.0.44",
          error: null,
          installed_but_broken: false,
          latest_authority: "npmmirror",
        },
      ]),
    ).toThrow("Grok 安装状态不可用");
  });

  it("rejects extra locator fields", () => {
    expect(() =>
      parseGrokToolSnapshot([
        {
          name: "grok",
          version: "1.0.5",
          latest_version: "1.0.6",
          error: null,
          installed_but_broken: false,
          url: "https://example.test",
        },
      ]),
    ).toThrow("Grok 安装状态不可用");
  });
});
