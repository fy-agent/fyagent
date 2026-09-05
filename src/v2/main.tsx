import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router-dom";

import { createAppRouter } from "./app/router";
import {
  prefetchPrimaryRoutes,
  preloadInitialPrimaryRoute,
} from "./app/primaryPages";
import { RootError } from "./app/RootError";
import "./app/styles/index.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("FyAgent V2 requires a root element.");
}

const root = createRoot(rootElement);

void preloadInitialPrimaryRoute(window.location.hash)
  .then(() => {
    const router = createAppRouter();
    root.render(
      <StrictMode>
        <RouterProvider router={router} />
      </StrictMode>,
    );
    prefetchPrimaryRoutes();
  })
  .catch(() => {
    root.render(<RootError />);
  });
