import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const read = (relativePath: string) =>
  fs.readFileSync(path.join(ROOT, relativePath), "utf8").replace(/\r\n/g, "\n");

const host = read("src-tauri/src/lib.rs");
const cargo = read("src-tauri/Cargo.toml");
const dialog = read("src/components/DeepLinkImportDialog.tsx");

const desktopTargetCfg =
  'any(target_os = "macos", target_os = "windows")';

describe("single-instance semantic activation contract", () => {
  it("applies the argv envelope bound only to Windows", () => {
    expect(host).toMatch(
      /#\[cfg\(target_os = "windows"\)\]\s+let args = match crate::windows_runtime::normalize_single_instance_args\(args\)/,
    );
    expect(host).not.toMatch(
      /#\[cfg\(not\(target_os = "windows"\)\)\]\s+let args = args;/,
    );
  });

  it("registers and destroys the single-instance plugin only on its Cargo desktop targets", () => {
    expect(cargo).toContain(
      `[target.'cfg(${desktopTargetCfg})'.dependencies]\ntauri-plugin-single-instance = "2"`,
    );
    expect(host).toContain(
      `#[cfg(${desktopTargetCfg})]\n    let builder = builder.plugin(tauri_plugin_single_instance::init`,
    );
    expect(host).toContain(
      `#[cfg(${desktopTargetCfg})]\n    tauri_plugin_single_instance::destroy(_app_handle);`,
    );
    expect(host).not.toContain(
      `#[cfg(not(${desktopTargetCfg}))]`,
    );
    expect(host).toContain(
      "let builder = tauri::Builder::default().plugin(activation_ready_plugin());",
    );
  });

  it("queues only parsed semantics until both renderer listeners are ready", () => {
    const semanticQueue = host.slice(
      host.indexOf("enum PendingActivation"),
      host.indexOf("fn emit_safe_deeplink_error"),
    );
    expect(semanticQueue).toContain("PendingActivation::DeepLink");
    expect(semanticQueue).toContain("PendingActivation::InvalidDeepLink");
    expect(host).toContain("MAX_PENDING_ACTIVATIONS: usize = 16");
    expect(semanticQueue).toContain("RejectedAtCapacity");
    expect(semanticQueue).toContain("bounded DoS policy");
    expect(semanticQueue).not.toMatch(/argv|Vec<String>|cwd/i);

    expect(host).toContain(
      'const FRONTEND_DEEPLINK_READY_EVENT: &str = "frontend-deeplink-ready"',
    );
    expect(host).toContain("mark_activation_renderer_ready(&activation_app)");
    expect(dialog).toContain("Promise.all([unlistenImport, unlistenError])");
    expect(dialog).toContain('await emit("frontend-deeplink-ready")');
  });

  it("does not rebuild lightweight mode for a non-focusing Windows rejection", () => {
    const wakePolicy = host.slice(
      host.indexOf("impl PendingActivation"),
      host.indexOf("struct ActivationInbox"),
    );
    const submission = host.slice(
      host.indexOf("fn submit_activation"),
      host.indexOf("fn emit_safe_deeplink_error"),
    );

    expect(wakePolicy).toContain("Self::Focus => true");
    expect(wakePolicy).toContain("=> *focus_main_window");
    expect(submission).toContain(
      "should_exit_lightweight_mode(was_lightweight, &activation)",
    );
    expect(submission).toContain("if should_exit_lightweight");
    const capacityRejection = submission.slice(
      submission.indexOf("ActivationEnqueueResult::RejectedAtCapacity"),
      submission.indexOf("if should_exit_lightweight"),
    );
    expect(capacityRejection).not.toContain("return;");
    expect(host).toContain('focus_main_window: cfg!(target_os = "macos")');
  });

  it("reserves full-queue priority for waking semantics without coupling wake to admission", () => {
    const inbox = host.slice(
      host.indexOf("impl ActivationInbox"),
      host.indexOf("fn activation_inbox"),
    );

    expect(inbox).toContain("activation.should_wake_main_window()");
    expect(inbox).toContain(
      ".position(|queued| !queued.should_wake_main_window())",
    );
    expect(inbox).toContain("self.pending.remove(index)");
    expect(host).toContain(
      "fn waking_activation_displaces_a_non_waking_item_at_capacity()",
    );
    expect(host).toContain(
      "fn capacity_rejection_does_not_change_a_waking_activation_exit_policy()",
    );
  });

  it("falls back to a connected primary or first monitor before clamping", () => {
    expect(host).toContain("window.available_monitors()");
    expect(host).toContain("window.primary_monitor()");
    expect(host).toContain(
      "fallback_monitor_index(&available_work_areas, primary)",
    );
    expect(host).toContain("window_layout::clamp_window_geometry(");
  });
});
