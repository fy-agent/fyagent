import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router-dom";

import { createAppRouter } from "./app/router";
import { prefetchPrimaryRoutes } from "./app/primaryPages";
import "./app/styles/index.css";

prefetchPrimaryRoutes();

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("FyAgent V2 requires a root element.");
}

const router = createAppRouter();

createRoot(rootElement).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
