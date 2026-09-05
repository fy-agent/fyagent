import { render } from "@testing-library/react";
import { expect, it } from "vitest";
import { FrostedSurface } from "@/v2/shared/ui/GlassMaterial";

it("keeps the same backing across material handoff without optical runtimes or content copies", () => {
  const view = render(<FrostedSurface enhanced={false} />);
  const backing = view.container.firstElementChild;
  const rim = backing?.firstElementChild;
  view.rerender(<FrostedSurface enhanced />);
  expect(view.container.firstElementChild).toBe(backing);
  expect(backing?.firstElementChild).toBe(rim);
  expect(backing).toHaveAttribute("aria-hidden", "true");
  expect(backing).toHaveAttribute("data-enhanced", "true");
  expect(
    view.container.querySelectorAll("canvas, svg, input, textarea"),
  ).toHaveLength(0);
});
