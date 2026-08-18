import { isRouteErrorResponse, useRouteError } from "react-router-dom";

function getErrorDescription(error: unknown): string {
  if (isRouteErrorResponse(error)) {
    return error.statusText || `路由请求失败（${error.status}）`;
  }

  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "请重新打开 FyAgent 后重试。";
}

export function RootError() {
  const error = useRouteError();

  return (
    <section
      className="fy-root-error"
      role="alert"
      aria-labelledby="root-error-title"
    >
      <h1 id="root-error-title">FyAgent 无法加载此页面</h1>
      <p>{getErrorDescription(error)}</p>
    </section>
  );
}
