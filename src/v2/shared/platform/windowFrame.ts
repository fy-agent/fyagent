import { createBrowserWindowFramePort } from "./browser/windowFrame";
import { detectRuntime } from "./runtime";
import { createTauriWindowFramePort } from "./tauri/windowFrame";
import type { RuntimeEnvironment, WindowFramePort } from "./types";

export function createWindowFramePort(
  environment: RuntimeEnvironment = detectRuntime(),
): WindowFramePort {
  return environment.isNative
    ? createTauriWindowFramePort(environment.platform)
    : createBrowserWindowFramePort();
}

export const windowFramePort = createWindowFramePort();
