import { describe, expect, it } from "vitest";
import { AGENT_CATALOG_IDS } from "@/shared/features/directory";
import {
  healthActionPath,
  healthAgentFromSearch,
  healthStatus,
} from "@/shared/features/health-presentation";
import { HEALTH_STALE_AFTER_MS } from "@/shared/features/health";
import { healthSnapshotFixture } from "../fixtures/health";

describe("health status and safe navigation", () => {
  const checkedAt = "2026-09-09T00:00:00Z";
  const now = Date.parse(checkedAt);

  it("distinguishes local readiness from unknown request history", () => {
    expect(
      healthStatus(healthSnapshotFixture("codex", {}, checkedAt), now),
    ).toBe("ready");
    expect(
      healthStatus(
        healthSnapshotFixture(
          "codex",
          { auth: { state: "unknown", reasonCode: "auth_unknown" } },
          checkedAt,
        ),
        now,
      ),
    ).toBe("needs_attention");
    expect(
      healthStatus(
        healthSnapshotFixture(
          "codex",
          {
            last_request: {
              state: "not_supported",
              reasonCode: "not_supported",
              action: null,
            },
          },
          checkedAt,
        ),
        now,
      ),
    ).toBe("ready");
  });

  it("prioritizes stale, blocking, missing configuration and attention without changing facts", () => {
    const snapshot = healthSnapshotFixture(
      "codex",
      {
        auth: { state: "attention" },
        configuration: { state: "not_configured" },
        installation: { state: "blocked" },
      },
      checkedAt,
    );
    expect(healthStatus(snapshot, now)).toBe("blocked");
    expect(healthStatus(snapshot, now, true)).toBe("stale");
    expect(healthStatus(snapshot, now + HEALTH_STALE_AFTER_MS)).toBe("stale");
    expect(snapshot.checkedAt).toBe(checkedAt);
    expect(
      healthStatus(
        healthSnapshotFixture(
          "codex",
          {
            configuration: { state: "not_configured" },
            auth: { state: "attention" },
          },
          checkedAt,
        ),
        now,
      ),
    ).toBe("not_configured");
  });

  it("accepts all seven closed Agent values and rejects ambiguous or executable query values", () => {
    for (const id of AGENT_CATALOG_IDS)
      expect(healthAgentFromSearch(`agent=${id}`)).toBe(id);
    for (const query of [
      "agent=codex&agent=codex",
      "agent=unknown",
      "agent=https://example.test",
      "agent=../../secret",
      "agent=claude",
      "",
    ])
      expect(healthAgentFromSearch(query)).toBeNull();
    expect(healthActionPath("refresh", "codex")).toBeNull();
    expect(healthActionPath("model_test", "claude-code")).toBe(
      "/models?target=claude",
    );
    expect(healthActionPath("authentication", "codex")).toBe(
      "/auth?view=connections&consumer=codex",
    );
    expect(healthActionPath("authentication", "claude-code")).toBe(
      "/agents?target=claude-code&section=models",
    );
    for (const id of AGENT_CATALOG_IDS)
      expect(healthActionPath("configuration", id)).toMatch(
        /^\/models\?target=[a-z-]+$/u,
      );
  });
});
