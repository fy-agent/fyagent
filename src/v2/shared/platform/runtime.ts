import type { RuntimeEnvironment, WindowPlatform } from "./types";

interface NavigatorIdentity {
  platform?: string;
  userAgent?: string;
  userAgentData?: {
    platform?: string;
  };
}

interface RuntimeScope {
  isTauri?: unknown;
  __TAURI_INTERNALS__?: unknown;
  navigator?: NavigatorIdentity;
}

type NativeWindowPlatform = Exclude<WindowPlatform, "browser">;

export function detectNativePlatform(
  navigatorIdentity?: NavigatorIdentity,
): NativeWindowPlatform {
  const platformIdentity = [
    navigatorIdentity?.userAgentData?.platform,
    navigatorIdentity?.platform,
    navigatorIdentity?.userAgent,
  ]
    .filter((value): value is string => typeof value === "string")
    .join(" ")
    .toLowerCase();

  if (/windows|win32|win64/.test(platformIdentity)) {
    return "windows";
  }

  if (/macintosh|mac os|macintel|macos/.test(platformIdentity)) {
    return "macos";
  }

  if (/linux|x11/.test(platformIdentity) && !/android/.test(platformIdentity)) {
    return "linux";
  }

  return "unknown";
}

export function detectRuntime(
  scope: RuntimeScope = globalThis,
): RuntimeEnvironment {
  const isNative =
    scope.isTauri === true ||
    (typeof scope.__TAURI_INTERNALS__ === "object" &&
      scope.__TAURI_INTERNALS__ !== null);

  if (!isNative) {
    return { isNative: false, platform: "browser" };
  }

  return {
    isNative: true,
    platform: detectNativePlatform(scope.navigator),
  };
}
