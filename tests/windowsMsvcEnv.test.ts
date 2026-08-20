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
  VCTOOLS_COMPONENT: string;
  msvcArchitecture(architecture: string): { arch: string; hostArch: string };
  vswhereCandidates(): string[];
  msvcRequirementHint(): string;
  findVsInstallation(input: {
    spawn: (command: string, args: string[], options: unknown) => SpawnResult;
    candidates: string[];
  }): { installationPath: string; vswhere: string };
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

function fakeVswhereFile(): string {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-vswhere-"));
  const file = path.join(directory, "vswhere.exe");
  fs.writeFileSync(file, "");
  return file;
}

function vswhereSpawn(stdout: string, status = 0): SpawnResult {
  return { status, stdout, stderr: "" };
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
  });

  it("locates a VS installation that satisfies the VC tools component", () => {
    const vswhere = fakeVswhereFile();
    const spawn = (command: string) =>
      command.endsWith("vswhere.exe")
        ? vswhereSpawn(`${FAKE_INSTALLATION}\n`)
        : vswhereSpawn("");
    const result = msvc.findVsInstallation({ spawn, candidates: [vswhere] });
    expect(result.installationPath).toBe(FAKE_INSTALLATION);
    expect(result.vswhere).toBe(vswhere);
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("fails with the actionable hint when no VC tools component is installed", () => {
    const vswhere = fakeVswhereFile();
    const spawn = () => vswhereSpawn("");
    expect(() =>
      msvc.findVsInstallation({ spawn, candidates: [vswhere] }),
    ).toThrow("Desktop development with C++");
    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("fails with the actionable hint when vswhere itself is absent", () => {
    const missing = path.join(
      os.tmpdir(),
      `fyagent-missing-${process.pid}.exe`,
    );
    const spawn = () => vswhereSpawn(FAKE_INSTALLATION);
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
    const environment = {
      INCLUDE: "C:\\VS\\VC\\include",
      LIB: "C:\\VS\\VC\\lib",
      PATH: "C:\\VS\\VC\\bin;C:\\Windows",
    };
    const calls: Array<{ command: string; args: string[] }> = [];
    const spawn = (command: string, args: string[]) => {
      calls.push({ command, args });
      if (command.endsWith("vswhere.exe")) {
        return vswhereSpawn(`${FAKE_INSTALLATION}\n`);
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

    fs.rmSync(path.dirname(vswhere), { recursive: true, force: true });
  });

  it("rejects a missing INCLUDE or LIB from the loaded environment", () => {
    const vswhere = fakeVswhereFile();
    const spawn = (command: string) => {
      if (command.endsWith("vswhere.exe")) {
        return vswhereSpawn(`${FAKE_INSTALLATION}\n`);
      }
      return {
        status: 0,
        stdout: JSON.stringify({ PATH: "C:\\Windows" }),
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
        return vswhereSpawn(`${FAKE_INSTALLATION}\n`);
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
});
