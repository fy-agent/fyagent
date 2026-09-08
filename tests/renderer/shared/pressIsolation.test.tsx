import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it } from "vitest";
import { styleEffect, useMotionValue } from "@/shared/ui/motion";

it("disposing another control never cancels the retained control's style subscription", async () => {
  const { result } = renderHook(
    () => [useMotionValue(1), useMotionValue(1)] as const,
  );
  const first = document.createElement("button");
  const second = document.createElement("button");
  const disposeFirst = styleEffect(first, { scale: result.current[0] });
  const disposeSecond = styleEffect(second, { scale: result.current[1] });
  try {
    act(() => result.current[1].set(0.98));
    await waitFor(() =>
      expect(second.style.transform).toContain("scale(0.98)"),
    );
    disposeFirst();
    act(() => result.current[1].set(0.96));
    await waitFor(() =>
      expect(second.style.transform).toContain("scale(0.96)"),
    );
    disposeFirst();
    act(() => result.current[1].set(1.004));
    await waitFor(() =>
      expect(second.style.transform).toContain("scale(1.004)"),
    );
  } finally {
    disposeFirst();
    disposeSecond();
  }
});
