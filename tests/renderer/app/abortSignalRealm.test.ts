import { transferableAbortController } from "node:util";
import { expect, it, vi } from "vitest";

it("keeps Node requests and jsdom listener cancellation functional without replacing fetch", () => {
  const controller = new AbortController();
  const request = new Request("http://localhost/", {
    signal: controller.signal,
  });
  const listener = vi.fn();
  const windowListener = vi.fn();
  window.addEventListener("pointerup", windowListener, {
    signal: controller.signal,
  });
  const node = document.createElement("button");
  node.addEventListener("click", listener, { signal: controller.signal });
  node.click();
  expect(listener).toHaveBeenCalledTimes(1);
  controller.abort("stop");
  window.dispatchEvent(new Event("pointerup"));
  expect(windowListener).not.toHaveBeenCalled();
  node.click();
  expect(listener).toHaveBeenCalledTimes(1);
  expect(request.signal.aborted).toBe(true);
  const cancelled = new AbortController();
  cancelled.abort();
  node.addEventListener("click", listener, { signal: cancelled.signal });
  node.click();
  expect(listener).toHaveBeenCalledTimes(1);
});

it("shares real cancellation across several DOM targets and preserves the native Request reason", () => {
  const controller = transferableAbortController();
  const request = new Request("https://example.invalid", {
    signal: controller.signal,
  });
  const first = document.createElement("button");
  const second = document.createElement("button");
  const listener = vi.fn();
  first.addEventListener("click", listener, { signal: controller.signal });
  second.addEventListener("click", listener, { signal: controller.signal });
  first.click();
  second.click();
  expect(listener).toHaveBeenCalledTimes(2);
  const reason = new Error("cancelled by the owner");
  controller.abort(reason);
  first.click();
  second.click();
  expect(listener).toHaveBeenCalledTimes(2);
  expect(request.signal.aborted).toBe(true);
  expect(request.signal.reason).toBe(reason);
});

it("does not register a listener when a directly supplied native signal is already aborted", () => {
  const controller = transferableAbortController();
  controller.abort("already finished");
  const button = document.createElement("button");
  const listener = vi.fn();
  button.addEventListener("click", listener, { signal: controller.signal });
  button.click();
  expect(listener).not.toHaveBeenCalled();
});
