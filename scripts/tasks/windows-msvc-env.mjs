#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

export const VCTOOLS_COMPONENT =
  "Microsoft.VisualStudio.Component.VC.Tools.x86.x64";

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

export function vswhereCandidates() {
  return [
    "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe",
    "C:\\Program Files\\Microsoft Visual Studio\\Installer\\vswhere.exe",
  ];
}

export function msvcRequirementHint() {
  return 'Install Visual Studio 2022 Build Tools with the "Desktop development with C++" workload (MSVC x64/x86 build tools and the Windows SDK).';
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
  spawn = spawnSync,
  candidates = vswhereCandidates(),
} = {}) {
  const vswhere = resolveVswhere(candidates);
  if (!vswhere) {
    throw new Error(msvcRequirementHint());
  }
  const result = spawn(
    vswhere,
    [
      "-latest",
      "-version",
      "[17.0,18.0)",
      "-products",
      "*",
      "-requires",
      VCTOOLS_COMPONENT,
      "-property",
      "installationPath",
    ],
    {
      encoding: "utf8",
      windowsHide: true,
    },
  );
  if (result.error) throw result.error;
  const installationPath = (result.stdout ?? "").trim();
  if (result.status !== 0 || installationPath === "") {
    throw new Error(msvcRequirementHint());
  }
  return { installationPath, vswhere };
}

export function resolveMsvcEnvironment({
  platform = process.platform,
  architecture = process.arch,
  nodeExecutable = process.execPath,
  spawn = spawnSync,
  candidates = vswhereCandidates(),
} = {}) {
  switch (platform) {
    case "win32":
      break;
    default:
      return null;
  }
  const { arch, hostArch } = msvcArchitecture(architecture);
  const { installationPath } = findVsInstallation({ spawn, candidates });
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
  if (typeof environment !== "object" || environment === null) {
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
