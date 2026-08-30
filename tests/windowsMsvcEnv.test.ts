import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");

type SpawnResult = {
  status?: number;
  stdout?: string;
  stderr?: string;
  error?: Error;
};

type WindowsMsvcEnvModule = {
  SUPPORTED_VISUAL_STUDIO_VERSION_RANGE: string;
  VCTOOLS_COMPONENTS: Readonly<Record<"x64" | "arm64", string>>;
  msvcArchitecture(architecture: string): { arch: string; hostArch: string };
  msvcToolsComponent(architecture: string): string;
  vswhereCandidates(): string[];
  msvcRequirementHint(): string;
  findVsInstallation(input: {
    architecture?: string;
    spawn: (command: string, args: string[], options: unknown) => SpawnResult;
    candidates: string[];
  }): {
    installationPath: string;
    installationVersion: string;
    component: string;
    vswhere: string;
  };
  resolveMsvcToolchain(input: {
    platform?: string;
    architecture?: string;
    nodeExecutable?: string;
    spawn?: (command: string, args: string[], options: unknown) => SpawnResult;
    candidates?: string[];
  }): {
    visualStudio: string;
    msvc: string;
    environment: Record<string, string>;
  } | null;
  resolveMsvcEnvironment(input: {
    platform?: string;
    architecture?: string;
    nodeExecutable?: string;
    spawn?: (command: string, args: string[], options: unknown) => SpawnResult;
    candidates?: string[];
  }): Record<string, string> | null;
};

let msvc: WindowsMsvcEnvModule;

beforeAll(async () => {
  msvc = (await import(
    /* @vite-ignore */ pathToFileURL(
      path.join(ROOT, "scripts", "tasks", "windows-msvc-env.mjs"),
    ).href
  )) as WindowsMsvcEnvModule;
});

const FAKE_INSTALLATION = "C:\\VS\\2022\\BuildTools";
const FAKE_INSTALLATION_VERSION = "17.14.36512.132";
const FAKE_MSVC_VERSION = "14.44.35207";

function fakeVswhereFile(): string {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-vswhere-"));
  const file = path.join(directory, "vswhere.exe");
  fs.writeFileSync(file, "");
  return file;
}

function vswhereSpawn(stdout: string, status = 0): SpawnResult {
  return { status, stdout, stderr: "" };
}

function vswhereJson(
  installationPath = FAKE_INSTALLATION,
  installationVersion = FAKE_INSTALLATION_VERSION,
): string {
  return JSON.stringify([{ installationPath, installationVersion }]);
}

function msvcEnvironment(
  overrides: Record<string, string> = {},
): Record<string, string> {
  return {
    INCLUDE: "C:\\VS\\VC\\include",
    LIB: "C:\\VS\\VC\\lib",
    PATH: "C:\\VS\\VC\\bin;C:\\Windows",
    VisualStudioVersion: "17.0",
    VCToolsVersion: FAKE_MSVC_VERSION,
    ...overrides,
  };
}

describe("windows-msvc-env module", () => {
  it("maps process architectures to VsDevCmd arch and host_arch", () => {
    expect(msvc.msvcArchitecture("x64")).toEqual({
      arch: "x64",
      hostArch: "x64",
    });
    expect(msvc.msvcArchitecture("arm64")).toEqual({
      arch: "arm64",
      hostArch: "arm64",
    });
    expect(() => msvc.msvcArchitecture("ia32")).toThrow(
      "Unsupported MSVC host architecture",
    );
  });

  it("maps host architectures to their native VC tools component", () => {
    expect(msvc.SUPPORTED_VISUAL_STUDIO_VERSION_RANGE).toBe("[17.0,19.0)");
    expect(msvc.msvcToolsComponent("x64")).toBe(
      "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
    );
    expect(msvc.msvcToolsComponent("arm64")).toBe(
      "Microsoft.VisualStudio.Component.VC.Tools.ARM64",
    );
    expect(msvc.VCTOOLS_COMPONENTS).toEqual({
      x64: "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
      arm64: "Microsoft.VisualStudio.Component.VC.Tools.ARM64",
    });
    expect(() => msvc.msvcToolsComponent("ia32")).toThrow(
      "Unsupported MSVC host architecture",
    );
  });

  it("lists the official vswhere candidates for x64 and arm64 hosts", () => {
    const candidates = msvc.vswhereCandidates();
    expect(candidates.length).toBeGreaterThan(0);
    for (const candidate of candidates) {
      expect(candidate).toContain("vswhere.exe");
      expect(candidate).toContain("Microsoft Visual Studio");
    }
  });

  it("produces an actionable hint naming the Desktop C++ workload", () => {
    expect(msvc.msvcRequirementHint()).toContain(
      "Desktop development with C++",
    );
    expect(msvc.msvcRequirementHint()).toContain("Visual Studio 2022");
    expect(msvc.msvcRequirementHint()).toContain("Visual Studio 2026");
  });

  it.each([
    ["x64", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64"],
    ["arm64", "Microsoft.VisualStudio.Component.VC.Tools.ARM64"],
  ])(
    "locates a bounded VS installation with the %s host component",
    (architecture, component) => {
    const vswhere = fakeVswhereFile();
      const calls: Array<{ command: string; args: string[] }> = [];
      const spawn = (command: string, args: string[]) => {
        calls.push({ command, args });
        return vswhereSpawn(vswhereJson());
      };
      const result = msvc.findVsInstallation({
        architecture,
        spawn,
        candidates: [vswhere],
      });
    expect(result.installationPath).toBe(FAKE_INSTALLATION);
      expect(result.installationVersion).toBe(FAKE_INSTALLATION_VERSION);
      expect(result.component).toBe(component);
    expect(result.vswhere).toBe(vswhere);
      expect(calls).toEqual([
        {
          command: vswhere,
          args: [
            "-latest",
            "-version",
            "[17.0,19.0)",
            "-products",
            "*",
            "-requires",
            component,
            "-format",
            "json",
            "-utf8",
          ],
        },
      ]);
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
    },
  );

  it("fails with the actionable hint when no VC tools component is installed", () => {
    const vswhere = fakeVswhereFile();
    const spawn = () => vswhereSpawn("[]");
    expect(() =>
      msvc.findVsInstallation({ spawn, candidates: [vswhere] }),
    ).toThrow("Desktop development with C++");
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("accepts UTF-8 JSON with a BOM and rejects malformed instance fields", () => {
    const vswhere = fakeVswhereFile();
    const bomSpawn = () => vswhereSpawn(`\uFEFF${vswhereJson()}`);
    expect(
      msvc.findVsInstallation({ spawn: bomSpawn, candidates: [vswhere] })
        .installationVersion,
    ).toBe(FAKE_INSTALLATION_VERSION);

    const malformedSpawn = () =>
      vswhereSpawn(
        JSON.stringify([
          { installationPath: 42, installationVersion: [17, 14] },
        ]),
      );
    expect(() =>
      msvc.findVsInstallation({
        spawn: malformedSpawn,
        candidates: [vswhere],
      }),
    ).toThrow("Desktop development with C++");
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("fails with the actionable hint when vswhere itself is absent", () => {
    const missing = path.join(
      os.tmpdir(),
      `fyagent-missing-${process.pid}.exe`,
    );
    const spawn = () => vswhereSpawn(vswhereJson());
    expect(() =>
      msvc.findVsInstallation({ spawn, candidates: [missing] }),
    ).toThrow("Desktop development with C++");
  });

  it("returns null on non-Windows hosts without probing", () => {
    let calls = 0;
    const spawn = () => {
      calls += 1;
      return vswhereSpawn("");
    };
    expect(
      msvc.resolveMsvcEnvironment({
        platform: "darwin",
        architecture: "x64",
        spawn,
      }),
    ).toBeNull();
    expect(calls).toBe(0);
  });

  it("loads the MSVC environment for win32 x64 through VsDevCmd", () => {
    const vswhere = fakeVswhereFile();
    const environment = msvcEnvironment();
    const calls: Array<{ command: string; args: string[] }> = [];
    const spawn = (command: string, args: string[]) => {
      calls.push({ command, args });
      if (command.endsWith("vswhere.exe")) {
        return vswhereSpawn(vswhereJson());
      }
      return { status: 0, stdout: JSON.stringify(environment), stderr: "" };
    };

    const result = msvc.resolveMsvcEnvironment({
      platform: "win32",
      architecture: "x64",
      nodeExecutable: "C:\\Program Files\\node\\node.exe",
      spawn,
      candidates: [vswhere],
    });
    expect(result).toEqual(environment);

    const cmdCall = calls.find(({ command }) => command === "cmd.exe");
    expect(cmdCall).toBeDefined();
    expect(cmdCall?.args[0]).toBe("/d");
    expect(cmdCall?.args[1]).toBe("/s");
    expect(cmdCall?.args[2]).toBe("/c");
    expect(cmdCall?.args[3]).toContain("VsDevCmd.bat");
    expect(cmdCall?.args[3]).toContain("-arch=x64");
    expect(cmdCall?.args[3]).toContain("-host_arch=x64");

    const toolchain = msvc.resolveMsvcToolchain({
      platform: "win32",
      architecture: "x64",
      nodeExecutable: "C:\\Program Files\\node\\node.exe",
      spawn,
      candidates: [vswhere],
    });
    expect(toolchain).toEqual({
      visualStudio: FAKE_INSTALLATION_VERSION,
      msvc: FAKE_MSVC_VERSION,
      environment,
    });

    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("rejects a missing INCLUDE or LIB from the loaded environment", () => {
    const vswhere = fakeVswhereFile();
    const spawn = (command: string) => {
      if (command.endsWith("vswhere.exe")) {
        return vswhereSpawn(vswhereJson());
      }
      return {
        status: 0,
        stdout: JSON.stringify({
          PATH: "C:\\Windows",
          VisualStudioVersion: "17.0",
          VCToolsVersion: FAKE_MSVC_VERSION,
        }),
        stderr: "",
      };
    };
    expect(() =>
      msvc.resolveMsvcEnvironment({
        platform: "win32",
        architecture: "x64",
        spawn,
        candidates: [vswhere],
      }),
    ).toThrow(/missing required variable/);
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("rejects an unparseable VsDevCmd environment", () => {
    const vswhere = fakeVswhereFile();
    const spawn = (command: string) => {
      if (command.endsWith("vswhere.exe")) {
        return vswhereSpawn(vswhereJson());
      }
      return { status: 0, stdout: "not-json", stderr: "" };
    };
    expect(() =>
      msvc.resolveMsvcEnvironment({
        platform: "win32",
        architecture: "x64",
        spawn,
        candidates: [vswhere],
      }),
    ).toThrow(/Unable to parse the Visual Studio C\+\+ environment/);
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("rejects a selected installation outside Visual Studio 2022 or 2026", () => {
    const vswhere = fakeVswhereFile();
    const spawn = () => vswhereSpawn(vswhereJson(FAKE_INSTALLATION, "19.0"));
    expect(() =>
      msvc.findVsInstallation({ spawn, candidates: [vswhere] }),
    ).toThrow("Visual Studio 2022 or Visual Studio 2026");
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("rejects MSVC or environment versions that contradict the selected installation", () => {
    const vswhere = fakeVswhereFile();
    const invalidMsvcSpawn = (command: string) =>
      command.endsWith("vswhere.exe")
        ? vswhereSpawn(vswhereJson())
        : vswhereSpawn(
            JSON.stringify(msvcEnvironment({ VCToolsVersion: "latest" })),
          );
    expect(() =>
      msvc.resolveMsvcToolchain({
        platform: "win32",
        architecture: "x64",
        spawn: invalidMsvcSpawn,
        candidates: [vswhere],
      }),
    ).toThrow("valid VCToolsVersion");

    const mismatchedVsSpawn = (command: string) =>
      command.endsWith("vswhere.exe")
        ? vswhereSpawn(vswhereJson())
        : vswhereSpawn(
            JSON.stringify(msvcEnvironment({ VisualStudioVersion: "18.0" })),
          );
    expect(() =>
      msvc.resolveMsvcToolchain({
        platform: "win32",
        architecture: "x64",
        spawn: mismatchedVsSpawn,
        candidates: [vswhere],
      }),
    ).toThrow("does not match the selected installation");
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });
});
