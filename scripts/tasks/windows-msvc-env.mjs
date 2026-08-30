#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const SUPPORTED_VISUAL_STUDIO_VERSION_RANGE = "[17.0,19.0)";

export const VCTOOLS_COMPONENTS = Object.freeze({
  x64: "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
  arm64: "Microsoft.VisualStudio.Component.VC.Tools.ARM64",
});

const VISUAL_STUDIO_VERSION = /^(?:17|18)(?:\.\d+){1,3}$/u;
const MSVC_VERSION = /^\d+(?:\.\d+){1,3}$/u;

function trimmedString(value) {
  return typeof value === "string" ? value.trim() : "";
}

export function msvcArchitecture(architecture) {
  switch (architecture) {
    case "x64":
      return { arch: "x64", hostArch: "x64" };
    case "arm64":
      return { arch: "arm64", hostArch: "arm64" };
    default:
      throw new Error(`Unsupported MSVC host architecture: ${architecture}`);
  }
}

export function msvcToolsComponent(architecture) {
  const component = VCTOOLS_COMPONENTS[architecture];
  if (!component) {
    throw new Error(`Unsupported MSVC host architecture: ${architecture}`);
  }
  return component;
}

export function vswhereCandidates() {
  return [
    "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe",
    "C:\\Program Files\\Microsoft Visual Studio\\Installer\\vswhere.exe",
  ];
}

export function msvcRequirementHint() {
  return 'Install Visual Studio 2022 or Visual Studio 2026 Build Tools with the "Desktop development with C++" workload, the native-host MSVC tools, and the Windows SDK.';
}

function resolveVswhere(candidates) {
  for (const candidate of candidates) {
    try {
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
        return candidate;
      }
    } catch {
      // Continue through candidate list exactly as a lookup would.
    }
  }
  return undefined;
}

export function findVsInstallation({
  architecture = process.arch,
  spawn = spawnSync,
  candidates = vswhereCandidates(),
} = {}) {
  const vswhere = resolveVswhere(candidates);
  if (!vswhere) {
    throw new Error(msvcRequirementHint());
  }
  const component = msvcToolsComponent(architecture);
  const result = spawn(
    vswhere,
    [
      "-latest",
      "-version",
      SUPPORTED_VISUAL_STUDIO_VERSION_RANGE,
      "-products",
      "*",
      "-requires",
      component,
      "-format",
      "json",
      "-utf8",
    ],
    {
      encoding: "utf8",
      windowsHide: true,
    },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(msvcRequirementHint());
  }
  let instances;
  try {
    instances = JSON.parse(
      (result.stdout ?? "").trim().replace(/^\uFEFF/u, ""),
    );
  } catch {
    throw new Error(msvcRequirementHint());
  }
  if (!Array.isArray(instances) || instances.length !== 1) {
    throw new Error(msvcRequirementHint());
  }
  const [instance] = instances;
  const installationPath = trimmedString(instance?.installationPath);
  const installationVersion = trimmedString(instance?.installationVersion);
  if (
    !installationPath ||
    !installationVersion ||
    !VISUAL_STUDIO_VERSION.test(installationVersion)
  ) {
    throw new Error(msvcRequirementHint());
  }
  return {
    installationPath,
    installationVersion,
    component,
    vswhere,
  };
}

function loadMsvcEnvironment({
  installationPath,
  architecture,
  nodeExecutable,
  spawn,
}) {
  const { arch, hostArch } = msvcArchitecture(architecture);
  const vsDevCmd = path.join(
    installationPath,
    "Common7",
    "Tools",
    "VsDevCmd.bat",
  );
  const dumpScript = "process.stdout.write(JSON.stringify(process.env))";
  const command =
    `call "${vsDevCmd}" -no_logo -arch=${arch} -host_arch=${hostArch} >nul` +
    ` && "${nodeExecutable}" -e "${dumpScript}"`;
  const result = spawn("cmd.exe", ["/d", "/s", "/c", command], {
    encoding: "utf8",
    windowsHide: true,
    windowsVerbatimArguments: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const detail = (result.stderr ?? result.stdout ?? "").trim();
    throw new Error(
      `Unable to load the Visual Studio C++ environment${detail ? `: ${detail}` : ""}`,
    );
  }
  let environment;
  try {
    environment = JSON.parse((result.stdout ?? "").trim());
  } catch (error) {
    throw new Error(
      `Unable to parse the Visual Studio C++ environment: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  if (
    typeof environment !== "object" ||
    environment === null ||
    Array.isArray(environment)
  ) {
    throw new Error(
      "Visual Studio C++ environment did not produce a valid object",
    );
  }
  const missing = ["INCLUDE", "LIB"].filter(
    (name) => typeof environment[name] !== "string" || environment[name] === "",
  );
  if (missing.length > 0) {
    throw new Error(
      `Visual Studio C++ environment is missing required variable(s): ${missing.join(", ")}`,
    );
  }
  return environment;
}

export function resolveMsvcToolchain({
  platform = process.platform,
  architecture = process.arch,
  nodeExecutable = process.execPath,
  spawn = spawnSync,
  candidates = vswhereCandidates(),
} = {}) {
  switch (platform) {
    case "win32":
      break;
    case "darwin":
    case "linux":
      return null;
    default:
      throw new Error(`Unsupported MSVC host platform: ${platform}`);
  }
  const installation = findVsInstallation({
    architecture,
    spawn,
    candidates,
  });
  const environment = loadMsvcEnvironment({
    installationPath: installation.installationPath,
    architecture,
    nodeExecutable,
    spawn,
  });
  const msvcVersion = trimmedString(environment.VCToolsVersion);
  const environmentVisualStudioVersion = trimmedString(
    environment.VisualStudioVersion,
  );
  if (!msvcVersion || !MSVC_VERSION.test(msvcVersion)) {
    throw new Error(
      "Visual Studio C++ environment did not report a valid VCToolsVersion",
    );
  }
  if (
    !environmentVisualStudioVersion ||
    !VISUAL_STUDIO_VERSION.test(environmentVisualStudioVersion) ||
    environmentVisualStudioVersion.split(".", 1)[0] !==
      installation.installationVersion.split(".", 1)[0]
  ) {
    throw new Error(
      "Visual Studio C++ environment version does not match the selected installation",
    );
  }
  return {
    visualStudio: installation.installationVersion,
    msvc: msvcVersion,
    environment,
  };
}

export function resolveMsvcEnvironment({
  platform = process.platform,
  architecture = process.arch,
  nodeExecutable = process.execPath,
  spawn = spawnSync,
  candidates = vswhereCandidates(),
} = {}) {
  return resolveMsvcToolchain({
    platform,
    architecture,
    nodeExecutable,
    spawn,
    candidates,
  })?.environment ?? null;
}

function isMain(importMetaUrl) {
  if (!process.argv[1]) return false;
  return pathToFileURL(path.resolve(process.argv[1])).href === importMetaUrl;
}

if (isMain(import.meta.url)) {
  try {
    if (process.argv.length !== 3 || process.argv[2] !== "--json") {
      throw new Error("Usage: node windows-msvc-env.mjs --json");
    }
    const toolchain = resolveMsvcToolchain();
    if (!toolchain) {
      throw new Error("Windows MSVC inspection requires a native Windows host");
    }
    console.log(
      JSON.stringify({
        visualStudio: toolchain.visualStudio,
        msvc: toolchain.msvc,
      }),
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
