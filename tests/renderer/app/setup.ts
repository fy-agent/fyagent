import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, vi } from "vitest";

const originalAddEventListener = window.EventTarget.prototype.addEventListener;
const originalWindowAddEventListener = window.addEventListener;
const originalScrollTo = window.scrollTo;
let consoleErrorGuard: ReturnType<typeof vi.spyOn> | null = null;

beforeAll(() => {
  // jsdom has no layout/scroll implementation. Motion's auto-height resolver
  // restores scroll after measurement; record that call without claiming a
  // real viewport moved. Browser regressions retain the actual scrolling API.
  window.scrollTo = vi.fn();
});

beforeEach(() => {
  // restoreMocks runs before each test, so a beforeAll spy would be removed
  // before the very first assertion. Reinstall this guard for every test.
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
  window.scrollTo = originalScrollTo;
});

// Vitest 4 bridges native signals into jsdom, so the old realm bridge is no
// longer needed. Its bridge currently misses signals aborted before listener
// registration; preserve the platform's no-registration behavior at both entry
// points without replacing Request/fetch or weakening jsdom's signal checks.
function isAbortedNativeSignal(options?: boolean | AddEventListenerOptions) {
  return (
    typeof options === "object" &&
    options?.signal instanceof AbortSignal &&
    options.signal.aborted
  );
}
window.EventTarget.prototype.addEventListener = function (
  type,
  callback,
  options,
) {
  if (isAbortedNativeSignal(options)) return;
  return originalAddEventListener.call(this, type, callback, options);
};
// Vitest also installs a bound own-property window listener for teardown.
window.addEventListener = function (
  type: string,
  callback: EventListenerOrEventListenerObject | null,
  options?: boolean | AddEventListenerOptions,
) {
  if (callback === null || isAbortedNativeSignal(options)) return;
  return originalWindowAddEventListener.call(window, type, callback, options);
};

afterEach(() => {
  cleanup();
  window.location.hash = "";
});
