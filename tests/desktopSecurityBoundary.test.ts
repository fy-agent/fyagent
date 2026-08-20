import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const capabilityPath = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "capabilities",
  "default.json",
);
const tauriConfigPath = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "tauri.conf.json",
);
const downloadManifestScriptPath = path.resolve(
  __dirname,
  "..",
  "scripts",
  "generate-download-manifest.mjs",
);
const gitAttributesPath = path.resolve(__dirname, "..", ".gitattributes");
const miscCommandsPath = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "src",
  "commands",
  "misc.rs",
);
const lifecycleJobsPath = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "src",
  "codex_desktop",
  "jobs.rs",
);
const settingsCommandsPath = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "src",
  "commands",
  "settings.rs",
);
const appLibraryPath = path.resolve(
  __dirname,
  "..",
  "src-tauri",
  "src",
  "lib.rs",
);
const rendererLifecyclePaths = [
  path.resolve(__dirname, "..", "src", "main.tsx"),
  path.resolve(__dirname, "..", "src", "components", "DatabaseUpgrade.tsx"),
];
const activeWindowsInstallDocs = [
  path.resolve(__dirname, "..", "README.md"),
  path.resolve(__dirname, "..", "README_EN.md"),
  path.resolve(__dirname, "..", "README_JA.md"),
  path.resolve(
    __dirname,
    "..",
    "docs",
    "user-manual",
    "en",
    "1-getting-started",
    "1.2-installation.md",
  ),
  path.resolve(
    __dirname,
    "..",
    "docs",
    "user-manual",
    "zh",
    "1-getting-started",
    "1.2-installation.md",
  ),
  path.resolve(
    __dirname,
    "..",
    "docs",
    "user-manual",
    "ja",
    "1-getting-started",
    "1.2-installation.md",
  ),
];

describe("desktop IPC capability and CSP boundary", () => {
  it("keeps generic opener and broad plugin defaults out of the renderer capability", () => {
    const capability = JSON.parse(fs.readFileSync(capabilityPath, "utf8")) as {
      windows: string[];
      permissions: string[];
    };

    expect(capability.windows).toEqual(["main"]);
    expect(capability.permissions).toContain("log:allow-log");
    expect(capability.permissions).toContain("dialog:allow-message");
    expect(capability.permissions).not.toContain("process:allow-exit");
    expect(capability.permissions).not.toContain("opener:default");
    expect(capability.permissions).not.toContain("dialog:default");
    expect(capability.permissions).not.toContain("log:default");
    expect(capability.permissions).not.toContain("process:default");
    expect(capability.permissions).not.toContain("process:allow-restart");
  });

  it("routes renderer exits through the fixed-code lifecycle command", () => {
    for (const sourcePath of rendererLifecyclePaths) {
      const source = fs.readFileSync(sourcePath, "utf8");
      expect(source).not.toContain("@tauri-apps/plugin-process");
      expect(source).not.toMatch(/\bexit\s*\(/u);
      expect(source).toContain('invoke("exit_app")');
    }
  });

  it("keeps exit and restart cleanup on one first-wins lifecycle owner", () => {
    const jobs = fs.readFileSync(lifecycleJobsPath, "utf8");
    const settings = fs.readFileSync(settingsCommandsPath, "utf8");
    const library = fs.readFileSync(appLibraryPath, "utf8");

    expect(jobs).toContain("enum ProcessLifecycleState");
    expect(jobs).toContain("Idle,");
    expect(jobs).toContain("Cleaning(ProcessLifecycleTransition)");
    expect(jobs).toContain("Finalizing(ProcessLifecycleTransition)");
    expect(jobs).toContain("ProcessLifecycleClaim::StartCleanup(requested)");
    expect(jobs).toContain("ProcessLifecycleClaim::CleanupInProgress(current)");
    expect(jobs).not.toContain("current.merge(requested)");

    expect(settings).toContain(
      "if let ProcessLifecycleClaim::StartCleanup(_) = receipt.claim",
    );
    expect(settings).not.toContain("app.exit(0)");
    expect(settings).not.toContain("app.restart()");

    expect(library).toContain("PRE_APP_PROCESS_LIFECYCLE");
    expect(library).toContain("ProcessLifecycleClaimReceipt");
    expect(library).toContain("ProcessLifecycleCoordinatorOrigin::PreApp");
    expect(library).toContain("ProcessLifecycleCoordinatorOrigin::Service");
    expect(library).toContain(
      "if !matches!(receipt.claim, ProcessLifecycleClaim::StartCleanup(_))",
    );
  });

  it("keeps the CSP explicit and asset protocol limited to an empty allowlist", () => {
    const config = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8")) as {
      app: {
        security: {
          assetProtocol: { enable: boolean; scope: string[] };
          csp: string;
        };
      };
    };
    const { assetProtocol, csp } = config.app.security;

    expect(assetProtocol).toEqual({ enable: true, scope: [] });
    expect(csp).toContain("default-src 'self'");
    expect(csp).toContain(
      "connect-src 'self' ipc: http://ipc.localhost https: http:",
    );
    expect(csp).toContain("img-src 'self' data: https: http:");
    expect(csp).not.toContain("*");
  });

  it("does not advertise or classify Windows Portable downloads", () => {
    const manifestScript = fs.readFileSync(downloadManifestScriptPath, "utf8");

    expect(manifestScript).not.toContain("Windows-Portable");
    for (const doc of activeWindowsInstallDocs) {
      expect(fs.readFileSync(doc, "utf8")).not.toContain("Windows-Portable");
    }
  });

  it("reserves desktop visual baselines for explicit Git LFS review", () => {
    const gitAttributes = fs.readFileSync(gitAttributesPath, "utf8");
    expect(gitAttributes).toContain("*.mjs text eol=lf");
    expect(gitAttributes).toContain(
      "tests/e2e/visual-baselines/**/*.png filter=lfs diff=lfs merge=lfs -text",
    );
  });

  it("fails closed before an elevated Windows release can probe or run user CLIs", () => {
    const source = fs.readFileSync(miscCommandsPath, "utf8");
    const versionCommand = source.indexOf("pub async fn get_tool_versions");
    const lifecycleCommand = source.indexOf(
      "pub async fn run_tool_lifecycle_action",
    );
    const installationProbe = source.indexOf(
      "pub async fn probe_tool_installations",
    );

    expect(source).toContain("ELEVATED_WINDOWS_CLI_BOUNDARY_MESSAGE");
    expect(source).toContain("formal_windows_build()");
    expect(source).toContain(
      "elevated_windows_cli_boundary_active_for(crate::windows_runtime::formal_windows_build())",
    );
    for (const commandStart of [
      versionCommand,
      lifecycleCommand,
      installationProbe,
    ]) {
      expect(commandStart).toBeGreaterThan(-1);
      const commandSource = source.slice(commandStart, commandStart + 1200);
      expect(commandSource).toContain(
        "if elevated_windows_cli_boundary_active()",
      );
    }
    expect(source).toContain(
      "Do not let a release build reach build_tool_lifecycle_command",
    );
  });

  it("keeps generic CLI install and update flows independent of package validation", () => {
    const source = fs.readFileSync(miscCommandsPath, "utf8");
    const start = source.indexOf("pub async fn run_tool_lifecycle_action");
    const end = source.indexOf("\n///", start);
    expect(start).toBeGreaterThan(-1);
    expect(end).toBeGreaterThan(start);

    const lifecycleCommand = source.slice(start, end);
    expect(lifecycleCommand).toContain("build_tool_lifecycle_command");
    expect(lifecycleCommand).toContain("run_elevated_cli_lifecycle_whitelist");
    expect(lifecycleCommand).not.toMatch(
      /codex_desktop|checksum|sha256|package_identity|verify_reader/iu,
    );
  });
});
