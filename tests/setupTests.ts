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
  resetProviderState();
  server.resetHandlers();
  clearTauriInvocations();
  vi.clearAllMocks();
});

afterAll(() => {
  server.close();
});
