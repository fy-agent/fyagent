import type { FeaturePorts } from "../features/ports";
import { createBrowserFeaturePorts } from "./browser/features";
import { detectRuntime } from "./runtime";
import { createTauriFeaturePorts } from "./tauri/features";

export function createFeaturePorts(): FeaturePorts {
  return detectRuntime().isNative
    ? createTauriFeaturePorts()
    : createBrowserFeaturePorts();
}
