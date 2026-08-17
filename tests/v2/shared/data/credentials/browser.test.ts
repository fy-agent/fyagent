import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

import { createTauriCredentialsPort } from "@/v2/shared/platform/tauri/credentials";
import {
  createBrowserCredentialsPort,
  credentialBrowserFixtures,
} from "@/v2/shared/data/credentials";

function collectStrings(value: unknown, bag: string[] = []): string[] {
  if (typeof value === "string") {
    bag.push(value);
    return bag;
  }
  if (Array.isArray(value)) {
    value.forEach((item) => collectStrings(item, bag));
    return bag;
  }
  if (value && typeof value === "object") {
    Object.values(value).forEach((item) => collectStrings(item, bag));
  }
  return bag;
}

describe("credentials browser fixtures", () => {
  it("contains no realistic secrets, password fields, or textarea copy", () => {
    const serialized = JSON.stringify(credentialBrowserFixtures);
    expect(serialized).not.toMatch(/sk-/);
    expect(serialized).not.toMatch(/ghp_/);
    expect(serialized).not.toMatch(/password/i);
    expect(serialized).not.toMatch(/textarea/i);
    expect(serialized).not.toMatch(/粘贴密钥/);
    expect(collectStrings(credentialBrowserFixtures).join("\n")).not.toMatch(
      /sk-|password|textarea|粘贴密钥/i,
    );
  });

  it("simultaneously demonstrates the four required owner states", async () => {
    const snapshot = await createBrowserCredentialsPort().listWorkspace();
    const byId = Object.fromEntries(
      snapshot.owners.map((row) => [row.owner.ownerId, row]),
    );
    const refs = Object.fromEntries(
      snapshot.refs.map((row) => [row.secretRef, row]),
    );

    expect(byId["alpha-ready"]?.bindingState.state).toBe("bound");
    expect(refs[byId["alpha-ready"].bindingState.secretRef]?.availability).toBe(
      "ready",
    );

    expect(byId["beta-legacy"]?.bindingState.state).toBe("legacy");
    expect(byId["beta-legacy"].bindingState).toMatchObject({
      state: "legacy",
      legacyState: "sourcesConflict",
    });

    expect(byId["gamma-unbound"]?.bindingState.state).toBe("unbound");

    expect(byId["delta-locked"]?.bindingState.state).toBe("bound");
    const lockedRef =
      refs[byId["delta-locked"].bindingState.secretRef];
    expect(lockedRef.availability).toBe("locked");
    expect(lockedRef.lock?.source).toBe("fyAgentPolicy");
    expect(lockedRef.issue?.action).toBe("unlockFyAgent");
    expect(lockedRef.issue?.lockSource).toBe("fyAgentPolicy");

    expect(byId["xi-syslock"]?.bindingState.state).toBe("bound");
    const backendLockedRef =
      refs[byId["xi-syslock"].bindingState.secretRef];
    expect(backendLockedRef.availability).toBe("locked");
    expect(backendLockedRef.lock?.source).toBe("backend");
    expect(backendLockedRef.issue?.action).toBe("unlockBackend");
    expect(backendLockedRef.issue?.lockSource).toBe("backend");

    expect(byId["theta-plan"]?.bindingState.state).toBe("unbound");
    expect(byId["kappa-expired"]?.bindingState.state).toBe("unbound");
    expect(byId["iota-discard"]?.bindingState.state).toBe("bound");
    expect(byId["iota-discard"].bindingState.secretRef).toBe(
      byId["alpha-ready"].bindingState.secretRef,
    );
  });

  it("ships the extra viewport fixtures without flipping clocks", () => {
    const snapshot = credentialBrowserFixtures;
    const refs = Object.fromEntries(
      snapshot.refs.map((row) => [row.secretRef, row]),
    );
    const owners = Object.fromEntries(
      snapshot.owners.map((row) => [row.owner.ownerId, row]),
    );

    const missing = refs[owners["epsilon-missing"].bindingState.secretRef];
    expect(missing.availability).toBe("missing");
    expect(missing.issue?.action).toBe("captureReplacement");

    const revoked = refs[owners["zeta-revoked"].bindingState.secretRef];
    expect(revoked.availability).toBe("revoked");
    expect(revoked.revocation?.source).toBe("userDelete");
    expect(revoked.availability).not.toBe("missing");

    const unavailable = refs[owners["eta-hardware"].bindingState.secretRef];
    expect(unavailable.availability).toBe("unavailable");
    expect(unavailable.issue?.backendUnavailableReason).toBe(
      "hardwareUnregistered",
    );

    const clean = snapshot.candidates.find(
      (item) => item.state === "verifiedPendingPlan" && !item.pendingTerminalDisposition,
    );
    expect(clean?.kind).toBe("newBinding");

    const pending = snapshot.candidates.find(
      (item) => item.pendingTerminalDisposition === "discarded",
    );
    expect(pending?.state).toBe("verifiedPendingPlan");
    expect(pending?.issue?.action).toBe("discardCandidate");

    const expired = snapshot.candidates.find((item) => item.state === "expired");
    expect(expired?.pendingTerminalDisposition).toBeUndefined();

    const cleanup = snapshot.candidates.find(
      (item) => item.state === "cleanupRequired",
    );
    expect(cleanup?.issue?.action).toBe("completeRecovery");

    expect(snapshot.secretDeleteImpact.impact.noFallback).toBe(true);
    expect(snapshot.secretDeleteImpact.impact.affectedOwners.length).toBeGreaterThanOrEqual(
      2,
    );
    expect(snapshot.providerDeleteReady.impact.secretRetained).toBe(true);
    expect(
      snapshot.providerDeleteReady.impact.separateSecretDeleteAction,
    ).toBe("get_secret_delete_impact");
    expect(snapshot.providerDeleteBlocked.status).toBe(
      "blockedLegacyResolutionRequired",
    );
    expect(
      "providerDeleteImpactId" in snapshot.providerDeleteBlocked.blocked,
    ).toBe(false);
    expect(snapshot.registeredBackends.every((item) => item.backend.kind !== "hardware")).toBe(
      true,
    );
  });

  it("keeps the Tauri adapter as a refusing stub", async () => {
    const port = createTauriCredentialsPort();
    expect(port.source).toBe("tauri-stub");
    await expect(port.listWorkspace()).rejects.toThrow(/must not invoke Tauri/);
    const source = readFileSync(
      path.resolve("src/v2/shared/platform/tauri/credentials.ts"),
      "utf8",
    );
    expect(source).not.toMatch(/@tauri-apps/);
    expect(source).not.toMatch(/invoke\(/);
  });
});
