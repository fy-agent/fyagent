import { createElement, StrictMode, useEffect } from "react";
import { render, waitFor } from "@testing-library/react";
import { afterAll, expect, it, vi } from "vitest";

const { emitMock } = vi.hoisted(() => ({
  emitMock: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({ emit: emitMock }));

type RuntimeGlobal = typeof globalThis & {
  isTauri?: boolean;
};

const runtimeGlobal = globalThis as RuntimeGlobal;
runtimeGlobal.isTauri = true;

afterAll(() => {
  Reflect.deleteProperty(runtimeGlobal, "isTauri");
});

it("emits the payload-free frontend ready event once across StrictMode and repeated calls", async () => {
  const { signalFrontendReady } = await import(
    "../../../src/v2/shared/platform/lifecycle"
  );

  function ReadyProbe() {
    useEffect(() => {
      void signalFrontendReady();
    }, []);

    return null;
  }

  render(createElement(StrictMode, null, createElement(ReadyProbe)));

  await waitFor(() => {
    expect(emitMock).toHaveBeenCalledTimes(1);
  });

  const firstSignal = signalFrontendReady();
  const strictModeRepeat = signalFrontendReady();

  expect(firstSignal).toBe(strictModeRepeat);
  await Promise.all([firstSignal, strictModeRepeat]);
  await signalFrontendReady();

  expect(emitMock).toHaveBeenCalledTimes(1);
  expect(emitMock).toHaveBeenCalledWith("frontend-deeplink-ready");
});
