import "@testing-library/jest-dom";
import { afterAll, afterEach, beforeAll, vi } from "vitest";
import { cleanup } from "@testing-library/react";
import { server } from "./msw/server";
import { resetProviderState } from "./msw/state";
import { clearTauriInvocations } from "./msw/tauriMocks";

beforeAll(() => {
  server.listen({ onUnhandledRequest: "warn" });
});

afterEach(() => {
  cleanup();
  // 页面数据的本地缓存（src/lib/localCache.ts）不能从一个用例带到下一个
  for (const key of Object.keys(localStorage)) {
    if (key.startsWith("cc-switch-cache:")) localStorage.removeItem(key);
  }
  resetProviderState();
  server.resetHandlers();
  clearTauriInvocations();
  vi.clearAllMocks();
});

afterAll(() => {
  server.close();
});
