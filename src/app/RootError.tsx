import { useFrontendReady } from "../shared/platform/useFrontendReady";
import { Button } from "../shared/ui/Button";

export function RootError() {
  useFrontendReady();
  return (
    <section
      className="fy-root-error"
      role="alert"
      aria-labelledby="root-error-title"
    >
      <h1 id="root-error-title">页面暂时无法打开</h1>
      <p>界面加载未完成。请重新加载后重试。</p>
      <Button onClick={() => window.location.reload()}>重新加载界面</Button>
    </section>
  );
}
