import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { MemoryRouter, useNavigate } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { PersistentSurface } from "@/v2/shared/ui/PersistentSurface";
import {
  usePersistentSearchParams,
  useStickyVisibleValue,
} from "@/v2/shared/ui/usePersistentSearchParams";

function SearchProbe() {
  const { visible, searchParams } = usePersistentSearchParams();
  return (
    <div>
      <span data-testid="visible">{String(visible)}</span>
      <span data-testid="target">{searchParams.get("target") ?? ""}</span>
    </div>
  );
}

function SearchFixture() {
  const [active, setActive] = useState(true);
  const navigate = useNavigate();
  return (
    <>
      <button
        type="button"
        onClick={() => {
          setActive(false);
          navigate("/agents?target=claude-code");
        }}
      >
        hide-and-leave
      </button>
      <PersistentSurface active={active}>
        <SearchProbe />
      </PersistentSurface>
    </>
  );
}

function StickyProbe({
  visible,
  explicit,
}: {
  visible: boolean;
  explicit: string | null;
}) {
  const value = useStickyVisibleValue(visible, explicit, "fallback");
  return <span data-testid="sticky">{value}</span>;
}

describe("usePersistentSearchParams", () => {
  it("freezes the last visible search while the surface is hidden", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={["/models?target=workbuddy"]}>
        <SearchFixture />
      </MemoryRouter>,
    );

    expect(screen.getByTestId("target")).toHaveTextContent("workbuddy");

    await user.click(screen.getByRole("button", { name: "hide-and-leave" }));

    expect(screen.getByTestId("visible")).toHaveTextContent("false");
    expect(screen.getByTestId("target")).toHaveTextContent("workbuddy");
  });
});

describe("useStickyVisibleValue", () => {
  it("keeps the last explicit value when hidden or when the URL omits it", () => {
    const view = render(<StickyProbe visible explicit="workbuddy" />);
    expect(screen.getByTestId("sticky")).toHaveTextContent("workbuddy");

    view.rerender(<StickyProbe visible={false} explicit={null} />);
    expect(screen.getByTestId("sticky")).toHaveTextContent("workbuddy");

    view.rerender(<StickyProbe visible explicit={null} />);
    expect(screen.getByTestId("sticky")).toHaveTextContent("workbuddy");

    view.rerender(<StickyProbe visible explicit="codex" />);
    expect(screen.getByTestId("sticky")).toHaveTextContent("codex");
  });
});
