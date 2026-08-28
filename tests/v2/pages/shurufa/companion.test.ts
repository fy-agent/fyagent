import { describe, expect, it } from "vitest";

import {
  canonicalChord,
  deviceSettingsError,
  INITIAL_MAPPINGS,
  mappingErrors,
  networkChipLabel,
  ssidLooksFiveG,
} from "@/v2/pages/shurufa/companion";

describe("shurufa companion helpers", () => {
  it("detects 5 GHz SSID names and keeps 2.4 GHz names honest", () => {
    expect(ssidLooksFiveG("Office-5G")).toBe(true);
    expect(ssidLooksFiveG("home 5ghz")).toBe(true);
    expect(ssidLooksFiveG("lab-24g")).toBe(false);
    expect(ssidLooksFiveG("room5guest")).toBe(false);
  });

  it("canonicalizes chords and rejects duplicates", () => {
    expect(canonicalChord(["tab", "ctrl"])).toBe("CTRL+TAB");
    const errors = mappingErrors([
      { input: "ENCODER_CW", displayName: "上一项", keys: ["CTRL", "TAB"] },
      { input: "ENCODER_CCW", displayName: "下一项", keys: ["TAB", "CTRL"] },
      INITIAL_MAPPINGS[2],
    ]);
    expect(errors.get("ENCODER_CW")).toMatch(/重复/);
    expect(errors.get("ENCODER_CCW")).toMatch(/重复/);
  });

  it("validates device settings bounds without echoing secrets", () => {
    expect(
      deviceSettingsError({
        version: 1,
        ssid: "",
        password: "secret",
        apiKey: "sf-secret",
        model: "",
      }),
    ).toBe("Wi-Fi 名称必须包含 1–32 个字符。");
    expect(
      deviceSettingsError({
        version: 1,
        ssid: "lab",
        password: "secret",
        apiKey: "sf-secret",
        model: "",
      }),
    ).toBeNull();
  });

  it("labels network chips with 5 GHz failure semantics", () => {
    expect(networkChipLabel("CONNECTED", "10.0.0.8", null, false)).toBe(
      "已连接 10.0.0.8",
    );
    expect(networkChipLabel("FAILED", "", "BAND", true)).toBe("失败 · 仅2.4G");
    expect(networkChipLabel("CONNECTING", "", null, true)).toBe(
      "连接中 · 疑似5G",
    );
  });
});
