import type { WindowFramePort } from "../types";

async function noop(): Promise<void> {
  return undefined;
}

export function createBrowserWindowFramePort(): WindowFramePort {
  return {
    isNative: false,
    platform: "browser",
    prepareFrame: noop,
    minimize: noop,
    toggleMaximize: noop,
    close: noop,
  };
}
