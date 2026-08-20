import { afterAll, expect, it, vi } from "vitest";

const { emitMock } = vi.hoisted(() => ({ emitMock: vi.fn() }));

vi.mock("@tauri-apps/api/event", () => ({ emit: emitMock }));

type RuntimeGlobal = typeof globalThis & {
  isTauri?: boolean;
  __TAURI_INTERNALS__?: unknown;
};

const runtimeGlobal = globalThis as RuntimeGlobal;
Reflect.deleteProperty(runtimeGlobal, "isTauri");
Reflect.deleteProperty(runtimeGlobal, "__TAURI_INTERNALS__");

afterAll(() => {
  Reflect.deleteProperty(runtimeGlobal, "isTauri");
  Reflect.deleteProperty(runtimeGlobal, "__TAURI_INTERNALS__");
});

it("does not emit the frontend ready event in a browser", async () => {
  const { signalFrontendReady } = await import(
    "../../../src/v2/shared/platform/lifecycle"
  );

  await signalFrontendReady();
  await signalFrontendReady();

  expect(emitMock).not.toHaveBeenCalled();
});
