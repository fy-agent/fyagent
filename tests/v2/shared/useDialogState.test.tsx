import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useDialogState } from "@/v2/shared/ui/useDialogState";

describe("conditional dialog session identity", () => {
  it("preserves the setter but creates a fresh session for the same item after dismissal", () => {
    const { result } = renderHook(() => useDialogState<string>());
    const setter = result.current[1];
    act(() => setter("same-item"));
    const firstSession = result.current[2];
    act(() => setter((value) => value));
    expect(result.current[2]).toBe(firstSession);
    act(() => setter(null));
    expect(result.current[0]).toBeNull();
    act(() => setter("same-item"));
    expect(result.current[2]).not.toBe(firstSession);
    expect(result.current[1]).toBe(setter);
  });
});
