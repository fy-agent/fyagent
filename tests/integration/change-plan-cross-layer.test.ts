import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  applyChangePlanOutcomeSchema,
  changeJobUpdatedEventSchema,
  changePlanSchema,
} from "@/lib/api/change-plan";

const root = process.cwd();
const fixture = JSON.parse(
  fs.readFileSync(
    path.join(root, "tests/fixtures/changePlanDtoContract.v1.json"),
    "utf8",
  ),
) as Record<string, unknown>;

describe("change-plan cross-layer contract", () => {
  it("parses the Rust-produced plan, apply outcome, and bounded event fixture", () => {
    expect(changePlanSchema.parse(fixture.plan)).toEqual(fixture.plan);
    expect(applyChangePlanOutcomeSchema.parse(fixture.applyOutcome)).toEqual(
      fixture.applyOutcome,
    );
    expect(changeJobUpdatedEventSchema.parse(fixture.event)).toEqual(
      fixture.event,
    );
  });

  it("registers every fixed command exactly once and exposes no direct Codex fallback", () => {
    const rust = fs.readFileSync(
      path.join(root, "src-tauri/src/lib.rs"),
      "utf8",
    );
    for (const command of [
      "create_codex_provider_switch_plan",
      "apply_change_plan",
      "get_change_job",
      "list_recoverable_change_jobs",
    ]) {
      expect(rust.match(new RegExp(`commands::${command}`, "g"))).toHaveLength(
        1,
      );
    }

    const hook = fs.readFileSync(
      path.join(root, "src/hooks/useProviderActions.ts"),
      "utf8",
    );
    const codexBoundary = hook.indexOf('if (activeApp === "codex")');
    const directMutation = hook.indexOf("switchProviderMutation.mutateAsync");
    expect(codexBoundary).toBeGreaterThan(-1);
    expect(directMutation).toBeGreaterThan(codexBoundary);
  });
});
