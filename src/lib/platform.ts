export type DesktopPlatform = "windows" | "macos" | "unknown";

// 轻量平台检测，避免在 SSR 或无 navigator 的环境报错。
// 只有明确识别的 Windows 和 macOS 会得到平台行为；其他值一律失败关闭。
export const detectDesktopPlatform = (): DesktopPlatform => {
  try {
    const ua = navigator.userAgent || "";
    const plat = (navigator.platform || "").toLowerCase();
    if (
      /windows|win32|win64/i.test(ua) ||
      plat === "win32" ||
      plat === "win64"
    ) {
      return "windows";
    }
    if (/mac/i.test(ua) || plat.includes("mac") || plat === "darwin") {
      return "macos";
    }
    return "unknown";
  } catch {
    return "unknown";
  }
};

export const isMac = (): boolean => detectDesktopPlatform() === "macos";

export const isWindows = (): boolean => detectDesktopPlatform() === "windows";

// 这些常量设计为通过 JSX 属性 spread 消费（`{...DRAG_REGION_ATTR}`），
// 因为 `data-tauri-drag-region` 是 wry 侧的 attribute 存在性检测，必须
// 完全不渲染属性才算禁用；空字符串或 "false" 仍会触发。
export const DRAG_REGION_ENABLED = isWindows() || isMac();

export const DRAG_REGION_ATTR: Record<string, unknown> = DRAG_REGION_ENABLED
  ? { "data-tauri-drag-region": true }
  : {};

export const DRAG_REGION_STYLE: Record<string, unknown> = DRAG_REGION_ENABLED
  ? { WebkitAppRegion: "drag" }
  : {};
