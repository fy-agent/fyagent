#!/usr/bin/env node

import process from "node:process";
import { run, usageBoolean } from "./lib.mjs";
import {
  VCTOOLS_COMPONENT,
  findVsInstallation,
  msvcRequirementHint,
} from "./windows-msvc-env.mjs";

export const REQUIREMENTS = Object.freeze({
  darwin: {
    commands: [
      ["git", ["--version"], "Install the Xcode command-line tools."],
      ["xcode-select", ["-p"], "Run xcode-select --install interactively."],
      ["xcrun", ["--find", "clang"], "Install the Xcode command-line tools."],
    ],
  },
  win32: {
    commands: [
      ["git", ["--version"], "Install Git for Windows."],
      [
        "vswhere.exe",
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
        msvcRequirementHint(),
      ],
      [
        "reg.exe",
        [
          "query",
          "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients",
          "/s",
          "/f",
          "WebView2 Runtime",
        ],
        "Install the Microsoft Edge WebView2 Evergreen Runtime.",
      ],
    ],
  },
  linux: {
    commands: [
      ["git", ["--version"], "Install Git."],
      [
        "pkg-config",
        ["--version"],
        "Install pkg-config and the current Tauri 2 desktop prerequisites for this host.",
      ],
      [
        "cc",
        ["--version"],
        "Install a C/C++ compiler and the current Tauri 2 desktop prerequisites for this host.",
      ],
    ],
  },
});

function inspect(platform) {
  const requirements = REQUIREMENTS[platform];
  if (!requirements) {
    return {
      ok: false,
      platform,
      checks: [
        {
          name: "supported-host",
          ok: false,
          hint: `Unsupported host platform: ${platform}`,
        },
      ],
    };
  }
  const checks = [];
  for (const [command, args, hint] of requirements.commands) {
    const result = probe(command, args);
    checks.push({
      name: `${command} ${args.join(" ")}`,
      ok: result.status === 0,
      hint: result.status === 0 ? undefined : hint,
    });
  }
  return { ok: checks.every((check) => check.ok), platform, checks };
}

function probe(command, args) {
  if (command === "vswhere.exe") {
    try {
      findVsInstallation();
      return { status: 0, stdout: "", stderr: "" };
    } catch (error) {
      return {
        status: 1,
        stdout: "",
        stderr: error instanceof Error ? error.message : String(error),
      };
    }
  }
  try {
    return run(command, args, { capture: true, allowFailure: true });
  } catch (error) {
    return {
      status: 1,
      stdout: "",
      stderr: error instanceof Error ? error.message : String(error),
    };
  }
}

const describeIndex = process.argv.indexOf("--describe-platform");
if (describeIndex >= 0) {
  const platform = process.argv[describeIndex + 1];
  const requirements = REQUIREMENTS[platform];
  if (!requirements) {
    console.error(`Unknown platform: ${platform ?? ""}`);
    process.exit(2);
  }
  console.log(JSON.stringify({ platform, requirements }, null, 2));
} else {
  const report = inspect(process.platform);
  if (usageBoolean("json") || process.argv.includes("--json")) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(`System prerequisites (${report.platform}):`);
    for (const check of report.checks) {
      console.log(`  ${check.ok ? "PASS" : "FAIL"} ${check.name}`);
      if (check.hint) console.log(`       ${check.hint}`);
    }
  }
  if (!report.ok) process.exitCode = 1;
}
