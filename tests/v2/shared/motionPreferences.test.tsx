import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useMediaQuery } from "@/v2/shared/ui/useMediaQuery";
import { useReducedMotion } from "@/v2/shared/ui/motion";

describe("live motion preference", () => {
  it("reacts to a system preference change and detaches when unmounted", () => {
    let matches = false;
    const listeners = new Set<() => void>();
    const addEventListener = vi.fn((_event: string, listener: () => void) => {
      listeners.add(listener);
    });
    const removeEventListener = vi.fn(
      (_event: string, listener: () => void) => {
        listeners.delete(listener);
      },
    );
    vi.spyOn(window, "matchMedia").mockImplementation((query) => ({
      media: query,
      get matches() {
        return matches;
      },
      onchange: null,
      addEventListener,
      removeEventListener,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(() => true),
    }));
    const { result, unmount } = renderHook(useReducedMotion);
    expect(result.current).toBe(false);
    act(() => {
      matches = true;
      listeners.forEach((listener) => listener());
    });
    expect(result.current).toBe(true);
    act(() => {
      matches = false;
      listeners.forEach((listener) => listener());
    });
    expect(result.current).toBe(false);
    unmount();
    expect(listeners.size).toBe(0);
    expect(removeEventListener).toHaveBeenCalledTimes(
      addEventListener.mock.calls.length,
    );
  });

  it("uses its declared fallback in a host without matchMedia", () => {
    vi.stubGlobal("matchMedia", undefined);
    try {
      const { result } = renderHook(() =>
        useMediaQuery("(min-width: 800px)", true),
      );
      expect(result.current).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
