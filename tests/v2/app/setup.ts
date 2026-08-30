import { transferableAbortController } from "node:util";
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, vi } from "vitest";

const nodeAbortController = transferableAbortController();
let consoleErrorGuard: ReturnType<typeof vi.spyOn> | null = null;

beforeAll(() => {
  const originalConsoleError = console.error.bind(console);
  consoleErrorGuard = vi
    .spyOn(console, "error")
    .mockImplementation((...arguments_: unknown[]) => {
      const message = arguments_
        .map((argument) =>
          typeof argument === "string" ? argument : String(argument),
        )
        .join(" ");
      if (
        message.includes("Warning: An update to") &&
        message.includes("was not wrapped in act")
      ) {
        throw new Error(`Unexpected React act warning: ${message}`);
      }
      originalConsoleError(...arguments_);
    });
});

afterAll(() => {
  consoleErrorGuard?.mockRestore();
  consoleErrorGuard = null;
});

try {
  new Request("http://localhost", {
    signal: new AbortController().signal,
  });
} catch {
  // React Router creates requests with AbortController. Vitest's jsdom realm
  // can replace that constructor while the repository deliberately retains
  // Node's native Request, so keep both sides of the test boundary in the
  // same native realm rather than replacing or weakening Request itself.
  Object.defineProperties(globalThis, {
    AbortController: {
      configurable: true,
      writable: true,
      value: nodeAbortController.constructor,
    },
    AbortSignal: {
      configurable: true,
      writable: true,
      value: nodeAbortController.signal.constructor,
    },
  });
}

afterEach(() => {
  cleanup();
  window.location.hash = "";
});
