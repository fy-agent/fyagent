import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import {
  createMemoryRouter,
  RouterProvider,
  useNavigate,
} from "react-router-dom";
import { expect, it, vi } from "vitest";
import { PersistentPrimaryOutlet } from "@/v2/app/PersistentPrimaryOutlet";

const { renders } = vi.hoisted(() => ({ renders: { skills: 0 } }));
vi.mock("@/v2/shared/platform/useFrontendReady", () => ({
  useFrontendReady: () => undefined,
}));
vi.mock("@/v2/app/primaryPages", () => ({
  primaryPages: {
    agents: () => <div>Agents</div>,
    auth: () => <div>Auth</div>,
    skills: () => {
      renders.skills += 1;
      return <div>Skills content</div>;
    },
  },
}));

it("does not rerender an already hidden page just because another route changes", async () => {
  function Shell() {
    const navigate = useNavigate();
    return (
      <>
        <button onClick={() => void navigate("/agents")}>Agents route</button>
        <button onClick={() => void navigate("/auth")}>Auth route</button>
        <PersistentPrimaryOutlet />
      </>
    );
  }
  const router = createMemoryRouter([{ path: "*", element: <Shell /> }], {
    initialEntries: ["/skills"],
  });
  render(<RouterProvider router={router} />);
  fireEvent.click(screen.getByText("Agents route"));
  await screen.findByText("Agents");
  const hiddenCount = renders.skills;
  fireEvent.click(screen.getByText("Auth route"));
  await screen.findByText("Auth");
  await waitFor(() => expect(renders.skills).toBe(hiddenCount));
  expect(screen.getByText("Skills content")).not.toBeVisible();
});
