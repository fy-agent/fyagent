import { getCurrentWindow } from "@tauri-apps/api/window";

import type { WindowFramePort, WindowPlatform } from "../types";

type NativeWindowPlatform = Exclude<WindowPlatform, "browser">;

interface NativeWindow {
  setDecorations(decorations: boolean): Promise<void>;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
}

export function createTauriWindowFramePort(
  platform: NativeWindowPlatform,
  appWindow: NativeWindow = getCurrentWindow(),
): WindowFramePort {
  let preparationPromise: Promise<void> | undefined;

  return {
    isNative: true,
    platform,
    prepareFrame() {
      preparationPromise ??=
        platform === "windows"
          ? Promise.resolve().then(() => appWindow.setDecorations(false))
          : Promise.resolve();

      return preparationPromise;
    },
    minimize: () => appWindow.minimize(),
    toggleMaximize: () => appWindow.toggleMaximize(),
    close: () => appWindow.close(),
  };
}
