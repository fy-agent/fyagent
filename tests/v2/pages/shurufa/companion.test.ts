import { describe, expect, it } from "vitest";

import {
  asrHeadline,
  asrReasonLabel,
  canonicalChord,
  deviceSettingsError,
  hydrateProfile,
  INITIAL_MAPPINGS,
  mappingErrors,
  networkChipLabel,
  recReasonLabel,
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

  it("hydrates a legacy three-mapping profile without dropping user chords", () => {
    const hydrated = hydrateProfile({
      version: 1,
      revision: "rev-old",
      serial: { port: "COM3", baud: 115200 },
      target: {
        processName: "notepad.exe",
        processPath: "C:\\\\Windows\\\\notepad.exe",
      },
      mappings: [
        {
          input: "ENCODER_CW",
          displayName: "自定义上一项",
          keys: ["CTRL", "TAB"],
        },
        {
          input: "ENCODER_CCW",
          displayName: "自定义下一项",
          keys: ["CTRL", "SHIFT", "TAB"],
        },
        { input: "ENCODER_PRESS", displayName: "自定义确认", keys: ["ENTER"] },
      ],
    });
    expect(hydrated.mappings.map((mapping) => mapping.input)).toEqual(
      INITIAL_MAPPINGS.map((mapping) => mapping.input),
    );
    expect(hydrated.mappings[0]?.displayName).toBe("自定义上一项");
    expect(hydrated.mappings[3]).toEqual(INITIAL_MAPPINGS[3]);
    expect(hydrated.mappings[4]).toEqual(INITIAL_MAPPINGS[4]);
  });

  it("exposes default button chords that collide with an existing user mapping", () => {
    const hydrated = hydrateProfile({
      version: 1,
      revision: "rev-old",
      serial: { port: "COM3", baud: 115200 },
      target: {
        processName: "notepad.exe",
        processPath: "C:\\\\Windows\\\\notepad.exe",
      },
      mappings: [
        { input: "ENCODER_CW", displayName: "上一项", keys: ["CTRL", "1"] },
        {
          input: "ENCODER_CCW",
          displayName: "下一项",
          keys: ["CTRL", "SHIFT", "TAB"],
        },
        { input: "ENCODER_PRESS", displayName: "确认动作", keys: ["ENTER"] },
      ],
    });
    const errors = mappingErrors(hydrated.mappings);
    expect(errors.get("ENCODER_CW")).toMatch(/重复/);
    expect(errors.get("BUTTON_A")).toMatch(/重复/);
  });

  it("projects ASR and REC failure reasons into Chinese headlines", () => {
    expect(asrHeadline("START", null, "DONE")).toBe("正在转写…");
    expect(asrHeadline("FAIL", "CANCEL", "DONE")).toBe("转写已停止");
    expect(asrHeadline("FAIL", "WIFI", null)).toBe("转写失败 · 未联网");
    expect(asrHeadline("DONE", null, "DONE")).toBe("转写完成");
    expect(asrHeadline(null, null, "ACTIVE")).toBe("录音中");
    expect(asrHeadline(null, null, null)).toBe("可录音");
    expect(asrReasonLabel("KEY")).toBe("缺少 Key 或模型");
    expect(recReasonLabel("WIFI")).toBe("未联网");
    expect(recReasonLabel("I2S")).toBe("麦克风未就绪");
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
