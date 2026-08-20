import { detectRuntime } from "./runtime";
import { emitFrontendDeeplinkReady } from "./tauri/lifecycle";

let readyPromise: Promise<void> | undefined;

export function signalFrontendReady(): Promise<void> {
  readyPromise ??= detectRuntime().isNative
    ? Promise.resolve().then(emitFrontendDeeplinkReady)
    : Promise.resolve();

  return readyPromise;
}
