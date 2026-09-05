import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const read = (name: string) =>
  fs.readFileSync(
    path.resolve(
      [
        "SavePlanWorkspace.tsx",
        "CodexSavePlanWorkspace.tsx",
        "WorkBuddySavePlanWorkspace.tsx",
      ].includes(name)
        ? "src/v2/pages/models/apply"
        : "src/v2/shared/features/change-plans-ui",
      name,
    ),
    "utf8",
  );

describe("Change Plan workflow ownership", () => {
  it("shares save orchestration and delegates all automatic job reads to Query", () => {
    for (const file of [
      "CodexSavePlanWorkspace.tsx",
      "WorkBuddySavePlanWorkspace.tsx",
    ]) {
      expect(read(file)).toContain("<SavePlanWorkspace");
      expect(read(file)).not.toMatch(
        /useState|useEffect|applyChangePlan|getChangeJob/u,
      );
    }
    for (const file of ["SavePlanWorkspace.tsx", "ChangePlanWorkspace.tsx"]) {
      expect(read(file)).toContain("useChangeJob(");
      expect(read(file)).not.toMatch(/setInterval|setTimeout|useMutation/u);
    }
    expect(read("useChangeJob.ts")).toContain("useQuery(");
    expect(read("useChangeJob.ts")).toContain("usePersistentVisibility");
    const models = fs.readFileSync("src/v2/pages/models/Page.tsx", "utf8");
    expect(models).not.toContain("<ChangePlanWorkspace");
    expect(
      fs.readFileSync("src/v2/pages/auth/CodexRequestSource.tsx", "utf8"),
    ).toContain("<ChangePlanWorkspace");
    expect(
      fs.readFileSync("src/v2/pages/agents/AgentConfiguration.tsx", "utf8"),
    ).not.toMatch(/CodexDesktopInstallerPanel|AgentInstallReadinessSection/);
  });
});
