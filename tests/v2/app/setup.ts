import { transferableAbortController } from "node:util";
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

const nodeAbortController = transferableAbortController();

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
