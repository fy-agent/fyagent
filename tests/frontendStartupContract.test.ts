import fs from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path: string) => fs.readFileSync(path, "utf8");

describe("main-window presentation ownership", () => {
  it("warms the initial module before rendering and never signals from the shell", () => {
    const main = read("src/v2/main.tsx");
    expect(main.indexOf("void preloadInitialPrimaryRoute")).toBeLessThan(
      main.indexOf("root.render("),
    );
    expect(main.indexOf("root.render(")).toBeLessThan(
      main.indexOf("prefetchPrimaryRoutes();"),
    );
    expect(read("src/v2/widgets/app-shell/AppShell.tsx")).not.toContain(
      "signalFrontendReady",
    );
    expect(read("src/v2/shared/platform/useFrontendReady.ts")).not.toMatch(
      /requestAnimationFrame|visibilityState|setTimeout/,
    );
    expect(read("src/v2/app/RootError.tsx")).toContain("useFrontendReady()");
  });

  it("keeps ordinary reveals behind the existing activation gate", () => {
    const host = read("src-tauri/src/lib.rs");
    const setupStart = host.indexOf("// 静默启动：");
    const setup = host.slice(
      setupStart,
      host.indexOf(".invoke_handler", setupStart),
    );
    expect(setup).toContain("prepare_main_webview(&window)");
    expect(setup).toContain("request_main_window_focus(app.handle())");
    expect(setup).not.toContain("window.show()");
    expect(setup).toContain("settings.silent_startup");
    expect(read("src-tauri/src/lightweight.rs")).not.toContain("window.show()");
    const tray = read("src-tauri/src/tray.rs");
    expect(
      tray.slice(
        tray.indexOf('"show_main" =>'),
        tray.indexOf('"lightweight_mode" =>'),
      ),
    ).toContain("crate::request_main_window_focus(app)");
    const recovery = host.slice(
      host.indexOf("fn schedule_frontend_recovery("),
      host.indexOf("fn emit_safe_deeplink_error("),
    );
    expect(recovery).toContain("inbox.can_recover(generation)");
    expect(recovery).toContain("inbox.finish_recovery(generation)");
    expect(recovery).toContain("window.reload()");
    expect(recovery).not.toMatch(/window\.show\(|mark_ready\(/);
  });
});
