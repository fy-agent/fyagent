import { describe, expect, it } from "vitest";

describe("renderer act-warning guard lifetime", () => {
  it.each([1, 2])(
    "rejects an act warning after per-test mock restore (test %i)",
    () => {
      expect(() =>
        console.error(
          "Warning: An update to %s inside a test was not wrapped in act(...).",
          "GuardProbe",
        ),
      ).toThrow("Unexpected React act warning:");
    },
  );
});
