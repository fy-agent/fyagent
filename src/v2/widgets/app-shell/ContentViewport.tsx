import { Outlet } from "react-router-dom";

export function ContentViewport() {
  return (
    <div className="fy-content-frame">
      <main
        className="fy-content-viewport"
        aria-label="内容承载区"
        data-testid="content-viewport"
      >
        <Outlet />
      </main>
    </div>
  );
}
