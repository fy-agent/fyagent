import { transferableAbortController } from "node:util";
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, vi } from "vitest";

const nodeAbortController = transferableAbortController();
const DOMAbortController = window.AbortController;
const DOMAbortSignal = window.AbortSignal;
const originalAddEventListener = window.EventTarget.prototype.addEventListener;
const originalWindowAddEventListener = window.addEventListener;
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
  window.EventTarget.prototype.addEventListener = originalAddEventListener;
  window.addEventListener = originalWindowAddEventListener;
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

// Backport the narrow jsdom/Node signal bridge used by Vitest's upstream
// environment fix (vitest-dev/vitest#8704). Native Request/fetch remain native;
// DOM listeners still receive an actual jsdom signal and real cancellation.
// Remove when the adopted test environment provides this bridge itself.
const domSignals = new WeakMap<AbortSignal, AbortController>();
function domListenerOptions(
  options?: boolean | AddEventListenerOptions,
): boolean | AddEventListenerOptions | undefined {
  if (
    typeof options === "object" &&
    options.signal &&
    !((options.signal as unknown) instanceof DOMAbortSignal)
  ) {
    const signal = options.signal;
    let controller = domSignals.get(signal);
    if (!controller) {
      controller = new DOMAbortController();
      const receiver = controller;
      if (signal.aborted) receiver.abort(signal.reason);
      else
        signal.addEventListener("abort", () => receiver.abort(signal.reason), {
          once: true,
        });
      domSignals.set(signal, controller);
    }
    return { ...options, signal: controller.signal };
  }
  return options;
}
window.EventTarget.prototype.addEventListener = function (
  type,
  callback,
  options,
) {
  return originalAddEventListener.call(
    this,
    type,
    callback,
    domListenerOptions(options),
  );
};
// Vitest also installs a bound own-property window listener for teardown.
window.addEventListener = function (
  type: string,
  callback: EventListenerOrEventListenerObject | null,
  options?: boolean | AddEventListenerOptions,
) {
  if (callback === null) return;
  return originalWindowAddEventListener.call(
    window,
    type,
    callback,
    domListenerOptions(options),
  );
};

afterEach(() => {
  cleanup();
  window.location.hash = "";
});
