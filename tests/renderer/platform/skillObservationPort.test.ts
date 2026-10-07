import { beforeEach, describe, expect, it, vi } from "vitest";

import { createSkillAssignments } from "@/shared/features/assignments";
import {
  parseObservedInstalledSkills,
  parseObservedUnmanagedSkills,
  SKILL_OBSERVATION_PAYLOAD_ERROR,
} from "@/shared/features/skills";
import { createSimpleFeaturePorts } from "@/shared/platform/tauri/feature-ports/simple";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function installed() {
  return {
    id: "local:fixture",
    name: "Fixture",
    directory: "fixture",
    path: "/fixture/skills/fixture",
    apps: createSkillAssignments(),
    installedAt: 1,
    updatedAt: 0,
    readOnly: false,
    readOnlyTargets: [] as string[],
  };
}
function unmanaged() {
  return {
    directory: "fixture",
    name: "Fixture",
    path: "/fixture/skills/fixture",
    foundIn: ["claude"],
    readOnly: false,
  };
}

beforeEach(() => {
  invoke.mockReset();
});

describe("Skills native observation boundary", () => {
  it("uses the two exact parameter-free observation commands and retains false", async () => {
    const rows = [installed()];
    const unmanagedRows = [unmanaged()];
    invoke.mockResolvedValueOnce(rows).mockResolvedValueOnce(unmanagedRows);
    const { skills } = createSimpleFeaturePorts();
    await expect(skills.getInstalled()).resolves.toEqual(rows);
    await expect(skills.scanUnmanaged()).resolves.toEqual(unmanagedRows);
    expect(invoke.mock.calls).toEqual([
      ["get_installed_skills"],
      ["scan_unmanaged_skills"],
    ]);
  });

  it("accepts the complete native nine-target observation independently of source readOnly", () => {
    const row = {
      ...installed(),
      readOnlyTargets: [
        "claude",
        "codex",
        "gemini",
        "grokbuild",
        "opencode",
        "hermes",
        "qoderwork",
        "trae-work",
        "workbuddy",
      ],
    };
    expect(parseObservedInstalledSkills([row])).toEqual([row]);
    expect(
      parseObservedInstalledSkills([{ ...row, readOnly: true }])[0].readOnly,
    ).toBe(true);
    expect(
      parseObservedUnmanagedSkills([{ ...unmanaged(), readOnly: true }])[0]
        .readOnly,
    ).toBe(true);
  });

  it.each(
    [undefined, null, {}, true, "[]", [null], [[]], [false]].map((payload) => ({
      payload,
    })),
  )("rejects malformed array or row shape %j", ({ payload }) => {
    expect(() => parseObservedInstalledSkills(payload)).toThrow(
      SKILL_OBSERVATION_PAYLOAD_ERROR,
    );
    expect(() => parseObservedUnmanagedSkills(payload)).toThrow(
      SKILL_OBSERVATION_PAYLOAD_ERROR,
    );
  });

  it.each([undefined, null, "false", 0, {}])(
    "rejects missing or malformed readOnly %j instead of treating it as writable",
    async (readOnly) => {
      invoke.mockResolvedValueOnce([{ ...installed(), readOnly }]);
      await expect(
        createSimpleFeaturePorts().skills.getInstalled(),
      ).rejects.toThrow(SKILL_OBSERVATION_PAYLOAD_ERROR);
      invoke.mockResolvedValueOnce([{ ...unmanaged(), readOnly }]);
      await expect(
        createSimpleFeaturePorts().skills.scanUnmanaged(),
      ).rejects.toThrow(SKILL_OBSERVATION_PAYLOAD_ERROR);
    },
  );

  it.each(
    [
      undefined,
      null,
      "claude",
      [false],
      ["claude", "claude"],
      ["unknown"],
      ["claude-desktop"],
      ["openclaw"],
      ["Gemini"],
      [" hermes"],
    ].map((readOnlyTargets) => ({ readOnlyTargets })),
  )(
    "rejects missing, duplicate or unknown readonly targets %j",
    ({ readOnlyTargets }) => {
      expect(() =>
        parseObservedInstalledSkills([{ ...installed(), readOnlyTargets }]),
      ).toThrow(SKILL_OBSERVATION_PAYLOAD_ERROR);
    },
  );

  it("rejects a legacy observation response while preserving base-only mutation DTO compatibility", async () => {
    const legacy: Record<string, unknown> = { ...installed() };
    delete legacy.readOnly;
    delete legacy.readOnlyTargets;
    invoke.mockResolvedValueOnce([legacy]);
    const { skills } = createSimpleFeaturePorts();
    await expect(skills.getInstalled()).rejects.toThrow(
      SKILL_OBSERVATION_PAYLOAD_ERROR,
    );
    invoke.mockResolvedValueOnce(legacy);
    await expect(
      skills.install(
        {
          key: "repo:fixture",
          name: "Fixture",
          description: "",
          directory: "fixture",
          repoOwner: "owner",
          repoName: "repo",
          repoBranch: "main",
        },
        "claude",
      ),
    ).resolves.toEqual(legacy);
    expect(invoke.mock.calls[1][0]).toBe("install_skill_unified");
  });

  it("preserves base fields and extension values without inventing observations", () => {
    const row = {
      ...installed(),
      repoBranch: "branch",
      extension: { retained: true },
    };
    expect(parseObservedInstalledSkills([row])[0]).toBe(row);
    expect(parseObservedInstalledSkills([])).toEqual([]);
    expect(parseObservedUnmanagedSkills([])).toEqual([]);
    expect(() =>
      parseObservedInstalledSkills([{ ...row, apps: null }]),
    ).toThrow(SKILL_OBSERVATION_PAYLOAD_ERROR);
    expect(() =>
      parseObservedUnmanagedSkills([{ ...unmanaged(), foundIn: null }]),
    ).toThrow(SKILL_OBSERVATION_PAYLOAD_ERROR);
  });
});
