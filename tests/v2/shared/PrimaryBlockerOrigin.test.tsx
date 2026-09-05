import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { Link, RouterProvider, createMemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import {
  PrimaryBlockerProvider,
  usePrimaryBlocker,
  usePrimaryBlockerOrigin,
  usePrimaryNavigationOrigin,
} from "@/v2/shared/ui/PrimaryBlocker";

function Guard() {
  const blocker = usePrimaryBlocker(true);
  const origin = usePrimaryBlockerOrigin();
  const capture = usePrimaryNavigationOrigin();
  return (
    <>
      <Link
        to="/next"
        onClick={(event) => capture(event.currentTarget, "/next")}
      >
        Owned navigation
      </Link>
      <Link
        to="/other"
        onClick={(event) => capture(event.currentTarget, "/wrong")}
      >
        Mismatched navigation
      </Link>
      <output data-testid="origin-state">
        {blocker.state === "blocked"
          ? (origin.current?.textContent ?? "neutral")
          : "idle"}
      </output>
      {blocker.state === "blocked" && (
        <button onClick={() => blocker.reset()}>Cancel</button>
      )}
    </>
  );
}

function setup() {
  const router = createMemoryRouter(
    [
      {
        path: "*",
        element: (
          <PrimaryBlockerProvider>
            <Guard />
          </PrimaryBlockerProvider>
        ),
      },
    ],
    { initialEntries: ["/start"] },
  );
  render(<RouterProvider router={router} />);
  return router;
}

describe("one-shot guarded navigation origin", () => {
  it("uses only a matching owned control and never changes route admission", async () => {
    const router = setup();
    fireEvent.click(screen.getByRole("link", { name: "Owned navigation" }));
    await waitFor(() =>
      expect(screen.getByTestId("origin-state")).toHaveTextContent(
        "Owned navigation",
      ),
    );
    expect(router.state.location.pathname).toBe("/start");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() =>
      expect(screen.getByTestId("origin-state")).toHaveTextContent("idle"),
    );
    await act(async () => {
      await router.navigate("/next");
    });
    expect(screen.getByTestId("origin-state")).toHaveTextContent("neutral");
    expect(router.state.location.pathname).toBe("/start");
  });

  it("does not attribute a programmatic or mismatched destination to an unrelated control", async () => {
    setup();
    fireEvent.click(
      screen.getByRole("link", { name: "Mismatched navigation" }),
    );
    await waitFor(() =>
      expect(screen.getByTestId("origin-state")).toHaveTextContent("neutral"),
    );
  });
});
