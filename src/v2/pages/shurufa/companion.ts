import type {
  CompanionDeviceSettings,
  CompanionInputId,
  CompanionMapping,
  CompanionNetwork,
  CompanionNetworkState,
  CompanionProfile,
  CompanionRuntime,
  CompanionRuntimeState,
  CompanionSnapshot,
} from "../../shared/features/ports";
import { DEFAULT_COMPANION_DEVICE_MODEL } from "../../shared/features/ports";

const MODIFIERS = ["CTRL", "ALT", "SHIFT"] as const;
const ALLOWED_PRIMARY = new Set([
  ...Array.from({ length: 26 }, (_, index) => String.fromCharCode(65 + index)),
  ...Array.from({ length: 10 }, (_, index) => String(index)),
  ...Array.from({ length: 24 }, (_, index) => `F${index + 1}`),
  "ENTER",
  "TAB",
  "ESC",
  "SPACE",
  "[",
  "]",
]);

const NAMED_PRIMARIES: Record<string, string> = {
  Enter: "ENTER",
  NumpadEnter: "ENTER",
  Tab: "TAB",
  Escape: "ESC",
  Space: "SPACE",
  BracketLeft: "[",
  BracketRight: "]",
};

export const USB_LINK_ID = "usb:ventured";
export const USB_LINK_BAUD = 115200;

export const INPUT_LABELS: Record<CompanionInputId, string> = {
  ENCODER_CW: "顺时针旋转",
  ENCODER_CCW: "逆时针旋转",
  ENCODER_PRESS: "GPIO8 外接确认/动作按钮",
  BUTTON_A: "GPIO10 下拉按键",
  BUTTON_B: "GPIO11 下拉按键",
};

export const INITIAL_MAPPINGS: CompanionMapping[] = [
  { input: "ENCODER_CW", displayName: "上一项", keys: ["CTRL", "TAB"] },
  {
    input: "ENCODER_CCW",
    displayName: "下一项",
    keys: ["CTRL", "SHIFT", "TAB"],
  },
  { input: "ENCODER_PRESS", displayName: "新建窗口", keys: ["CTRL", "SHIFT", "N"] },
  { input: "BUTTON_A", displayName: "新建", keys: ["CTRL", "N"] },
  { input: "BUTTON_B", displayName: "确认动作", keys: ["ENTER"] },
];

export const RUNTIME_STATE_LABELS: Record<CompanionRuntimeState, string> = {
  STOPPED: "已停止",
  DRY_RUN: "演练模式",
  LIVE: "实时模式",
};

export const NETWORK_STATE_LABELS: Record<CompanionNetworkState, string> = {
  UNKNOWN: "未连接",
  DISCONNECTED: "未连接",
  CONNECTING: "连接中",
  CONNECTED: "已连接",
  FAILED: "失败",
};

export const WIFI_BAND_HINT =
  "开发板只支持 2.4GHz Wi-Fi。5G 热点搜不到，也不会变成已连接。";
export const WIFI_FIVE_G_ALERT =
  "当前名称像 5G 热点，开发板连不上。请改用 2.4GHz 名称。";
export const WIFI_CONNECTING_STUCK_HINT =
  "仍在连接中：若这是 5G 热点，开发板搜不到，请改用 2.4GHz 名称。";
export const MIC_REC_HINT =
  "未联网时不能录音。手从远(~200mm)收到近(80-120mm)开始录音，从近收到远结束。GPIO9 仍可按住录音。转写中再按 GPIO9 或再做一次远→近会停止转写。开关有短缓冲，避免测距抖动连开连关。已联网且填写设备 Key 后，板端会把转写回传到这里，再自动交给输入法 Agent。";
export const SENSOR_HINT =
  "GPIO16 人体感应只判断座位是否有人，响应较慢。VL53L0X（SDA=GPIO4、SCL=GPIO5）用手势控录音：朝天 20-70mm 视为无效，远→近开，近→远关。";
export const CLOUD_OPTIONAL_HINT =
  "SiliconFlow API Key 可留空只测 Wi-Fi。转写默认 XingChenAGI/XingChenASR-V3.2-Ultra，也可自行填写其他模型。";
export const CLOUD_MODELS = [DEFAULT_COMPANION_DEVICE_MODEL] as const;

export const EMPTY_NETWORK: CompanionNetwork = {
  state: "UNKNOWN",
  ssid: "",
  ip: "",
  rssi: null,
  reason: null,
  pingHost: null,
  pingOk: null,
  pingMs: null,
  pingLost: null,
  pingSent: null,
  lastLog: null,
  beats: null,
  recState: null,
  recMs: null,
  recSamples: null,
  recRms: null,
  recPeak: null,
  recSilence: null,
  recReason: null,
  asrState: null,
  asrText: null,
  asrReason: null,
  pir: null,
  tofMm: null,
  sensorState: null,
};

export const EMPTY_RUNTIME: CompanionRuntime = {
  state: "STOPPED",
  liveEnabled: false,
  lastEvent: "尚无事件。",
  gapMissed: null,
  network: EMPTY_NETWORK,
};

export const DEFAULT_DEVICE_SETTINGS: CompanionDeviceSettings = {
  version: 1,
  ssid: "",
  password: "",
  apiKey: "",
  model: DEFAULT_COMPANION_DEVICE_MODEL,
};

export const EMPTY_PROFILE: CompanionProfile = {
  version: 1,
  revision: null,
  serial: { port: USB_LINK_ID, baud: USB_LINK_BAUD },
  target: null,
  mappings: INITIAL_MAPPINGS,
};

export function emptyCompanionSnapshot(): CompanionSnapshot {
  return {
    ports: [],
    profile: null,
    device: DEFAULT_DEVICE_SETTINGS,
    runtime: EMPTY_RUNTIME,
    lastAsrSeq: null,
    lastAsrAdmission: "none",
    lastAsrError: null,
  };
}

export function hydrateProfile(
  profile: CompanionProfile | null,
): CompanionProfile {
  if (!profile) return EMPTY_PROFILE;
  const byInput = new Map(
    profile.mappings.map((mapping) => [mapping.input, mapping]),
  );
  return {
    ...profile,
    serial: {
      port: USB_LINK_ID,
      baud: USB_LINK_BAUD,
    },
    mappings: INITIAL_MAPPINGS.map(
      (fallback) => byInput.get(fallback.input) ?? fallback,
    ),
  };
}

interface ParsedChord {
  modifiers: string[];
  primaries: string[];
}

export function parseChordTokens(
  tokens: readonly string[],
): ParsedChord | null {
  const normalized = tokens.map((token) => token.trim().toUpperCase());
  if (
    normalized.length === 0 ||
    normalized.length > 4 ||
    normalized.some((token) => token.length === 0)
  ) {
    return null;
  }
  if (new Set(normalized).size !== normalized.length) return null;
  const selectedModifiers = MODIFIERS.filter((modifier) =>
    normalized.includes(modifier),
  );
  const primaries = normalized.filter(
    (token) => !MODIFIERS.includes(token as (typeof MODIFIERS)[number]),
  );
  if (
    primaries.length === 0 ||
    primaries.some((token) => !ALLOWED_PRIMARY.has(token))
  ) {
    return null;
  }
  return { modifiers: selectedModifiers, primaries };
}

export function canonicalChord(tokens: readonly string[]): string | null {
  const parsed = parseChordTokens(tokens);
  return parsed ? [...parsed.modifiers, ...parsed.primaries].join("+") : null;
}

export function chordIdentity(tokens: readonly string[]): string | null {
  const parsed = parseChordTokens(tokens);
  return parsed
    ? [...parsed.modifiers, ...[...parsed.primaries].sort()].join("+")
    : null;
}

export function primaryFromKeyboardEvent(
  event: Pick<KeyboardEvent, "code" | "key">,
): string | null {
  if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3);
  if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
  if (/^Numpad[0-9]$/.test(event.code)) return event.code.slice(6);
  const functionKey = event.code.match(/^F([0-9]{1,2})$/);
  if (functionKey) {
    const number = Number(functionKey[1]);
    if (number >= 1 && number <= 24) return `F${number}`;
  }
  if (event.code === "BracketLeft" || event.key === "[" || event.key === "{") {
    return "[";
  }
  if (event.code === "BracketRight" || event.key === "]" || event.key === "}") {
    return "]";
  }
  return NAMED_PRIMARIES[event.code] ?? NAMED_PRIMARIES[event.key] ?? null;
}

export function modifiersFromKeyboardEvent(
  event: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey">,
): string[] {
  return [
    event.ctrlKey ? "CTRL" : null,
    event.altKey ? "ALT" : null,
    event.shiftKey ? "SHIFT" : null,
  ].filter((token): token is string => token !== null);
}

export function displayNameError(name: string): string | null {
  const trimmed = name.trim();
  const characters = Array.from(trimmed).length;
  if (characters < 1 || characters > 40) return "名称必须包含 1–40 个字符。";
  if (/\p{Cc}/u.test(trimmed)) return "名称不能包含控制字符。";
  return null;
}

export function mappingErrors(
  mappings: readonly CompanionMapping[],
): Map<CompanionInputId, string> {
  const errors = new Map<CompanionInputId, string>();
  const chords = new Map<string, CompanionInputId>();
  for (const mapping of mappings) {
    const nameError = displayNameError(mapping.displayName);
    if (nameError) errors.set(mapping.input, nameError);
    const chord = canonicalChord(mapping.keys);
    const identity = chordIdentity(mapping.keys);
    if (!chord || !identity) {
      errors.set(
        mapping.input,
        "请按下至少一个允许的主键，并可同时按 CTRL/ALT/SHIFT，总共最多四个键。",
      );
      continue;
    }
    const duplicate = chords.get(identity);
    if (duplicate) {
      errors.set(mapping.input, `与 ${duplicate} 重复：${chord}。`);
      errors.set(duplicate, `与 ${mapping.input} 重复：${chord}。`);
    } else {
      chords.set(identity, mapping.input);
    }
  }
  return errors;
}

function boundedField(
  value: string,
  min: number,
  max: number,
  label: string,
): string | null {
  const characters = Array.from(value).length;
  if (characters < min || characters > max) {
    return `${label}必须包含 ${min}–${max} 个字符。`;
  }
  if (/\p{Cc}/u.test(value)) return `${label}不能包含控制字符。`;
  return null;
}

export function deviceSettingsError(
  settings: CompanionDeviceSettings,
): string | null {
  const model = settings.model.trim();
  return (
    boundedField(settings.ssid, 1, 32, "Wi-Fi 名称") ??
    boundedField(settings.password, 0, 64, "Wi-Fi 密码") ??
    boundedField(settings.apiKey, 0, 256, "API Key") ??
    (model === "" ? null : boundedField(model, 1, 64, "转写模型"))
  );
}

export function ssidLooksFiveG(ssid: string): boolean {
  return /5\s*g(?:hz)?(?![0-9a-z])/i.test(ssid);
}

export function asrReasonLabel(reason: string | null): string | null {
  switch (reason) {
    case "CANCEL":
      return "已取消";
    case "BUSY":
      return "转写进行中";
    case "WIFI":
      return "未联网";
    case "KEY":
      return "缺少 Key 或模型";
    case "AUTH":
      return "鉴权失败";
    case "FORMAT":
      return "音频格式被拒";
    case "HTTP":
      return "上传失败";
    case "MEM":
      return "内存不足";
    case "I2S":
      return "麦克风未就绪";
    default:
      return null;
  }
}

export function asrHeadline(
  asrState: string | null,
  asrReason: string | null,
  recState: string | null = null,
): string {
  if (asrState === "START") return "正在转写…";
  if (asrState === "FAIL" && asrReason === "CANCEL") return "转写已停止";
  if (asrState === "FAIL") {
    const label = asrReasonLabel(asrReason);
    return label ? `转写失败 · ${label}` : "转写失败";
  }
  if (asrState === "DONE") return "转写完成";
  if (recState === "START" || recState === "ACTIVE") return "录音中";
  if (recState === "FAIL") return "录音失败";
  return "可录音";
}

export function recReasonLabel(reason: string | null): string | null {
  switch (reason) {
    case "WIFI":
      return "未联网";
    case "I2S":
      return "麦克风未就绪";
    case "BUSY":
      return "转写进行中";
    default:
      return null;
  }
}

export function recStateLabel(state: string | null): string | null {
  switch (state) {
    case "START":
    case "ACTIVE":
      return "录音中";
    case "DONE":
      return "录音完成";
    case "FAIL":
      return "录音失败";
    default:
      return null;
  }
}

export function networkReasonLabel(reason: string | null): string | null {
  switch (reason) {
    case "BAND":
      return "开发板仅支持 2.4GHz，当前名称像 5G 热点";
    case "NO_AP":
      return "找不到这个热点；请确认 2.4GHz 名称";
    case "AUTH":
      return "认证失败，请检查密码";
    case "TIMEOUT":
      return "连接超时";
    case "UNKNOWN":
      return "联网失败";
    default:
      return null;
  }
}

export function networkChipLabel(
  state: CompanionNetworkState,
  ip: string,
  reason: string | null,
  looksFiveG: boolean,
): string {
  if (state === "CONNECTED" && ip)
    return `${NETWORK_STATE_LABELS[state]} ${ip}`;
  if (reason === "BAND" || (looksFiveG && state === "FAILED")) {
    return "失败 · 仅2.4G";
  }
  if (looksFiveG && state !== "CONNECTED") {
    return `${NETWORK_STATE_LABELS[state]} · 疑似5G`;
  }
  return NETWORK_STATE_LABELS[state];
}

const RUNTIME_PHRASES: ReadonlyArray<readonly [string, string]> = [
  [
    "Dry-run started. No dispatcher constructed.",
    "已启动演练模式；未创建输入派发器。",
  ],
  ["Live enabled for this process only.", "仅为当前进程启用实时权限。"],
  ["No event yet.", "尚无事件。"],
  ["runtime is stopped", "运行已停止"],
  [
    "stop the active runtime before changing configuration",
    "请先停止当前运行，再修改配置",
  ],
  ["a valid saved profile is required", "需要有效且已保存的配置"],
  ["input is unmapped", "输入未映射"],
  ["profile mapping is invalid", "映射配置无效"],
  ["serial input stopped", "USB 输入已停止"],
  ["foreground target did not match", "前台目标不匹配"],
  ["foreground restore target is missing", "前台恢复目标不存在"],
  ["foreground restore was rejected", "前台恢复被拒绝"],
  ["keyboard state is not clear", "键盘按键状态不干净"],
  ["input dispatch rejected", "输入派发被拒绝"],
  [" · dry-run", " · 演练模式"],
  [" · live", " · 实时模式"],
  [" · dispatched", " · 已派发"],
  [" · rejected", " · 已拒绝"],
];

export function formatRuntimeText(value: string): string {
  return RUNTIME_PHRASES.reduce(
    (formatted, [english, chinese]) => formatted.split(english).join(chinese),
    value,
  );
}

export function asrAdmissionLabel(
  admission: CompanionSnapshot["lastAsrAdmission"],
): string | null {
  switch (admission) {
    case "start":
      return "转写开始";
    case "fail":
      return "转写失败";
    case "empty":
      return "转写为空，未进入 Agent";
    case "admitted":
      return "已交给输入法 Agent";
    case "duplicate":
      return "重复转写已忽略";
    case "busy":
      return "Agent 忙碌，本轮未进入生成";
    default:
      return null;
  }
}
