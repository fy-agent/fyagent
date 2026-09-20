import { expect, it, vi } from "vitest";
import { createTauriFeaturePorts } from "@/shared/platform/tauri/features";

const calls = vi.hoisted(() => ({
  projectsLoaded: vi.fn(),
  kitsLoaded: vi.fn(),
  verificationLoaded: vi.fn(),
  modelsLoaded: vi.fn(),
  recoveryLoaded: vi.fn(),
  providerSummary: vi.fn(async () => []),
  listRecoveries: vi.fn(async () => []),
  listProjects: vi.fn(async () => []),
  previewKit: vi.fn(async () => ({ previewId: "fixture" })),
  cancelVerification: vi.fn(async () => true),
  exportHandoff: vi.fn(async () => false),
}));
vi.mock("@/shared/platform/tauri/feature-ports/projects", () => {
  calls.projectsLoaded();
  return { createProjectsPort: () => ({ list: calls.listProjects }) };
});
vi.mock("@/shared/platform/tauri/feature-ports/delivery-kits", () => {
  calls.kitsLoaded();
  return {
    createDeliveryKitsPort: () => ({ previewBuiltin: calls.previewKit }),
  };
});
vi.mock("@/shared/platform/tauri/feature-ports/verification", () => {
  calls.verificationLoaded();
  return {
    createVerificationPort: () => ({
      cancel: calls.cancelVerification,
      export: calls.exportHandoff,
    }),
  };
});

vi.mock("@/shared/platform/tauri/feature-ports/models", () => {
  calls.modelsLoaded();
  return {
    createModelFeaturePorts: () => ({
      providers: { getSummary: calls.providerSummary },
    }),
  };
});
vi.mock("@/shared/platform/tauri/feature-ports/configRecovery", () => {
  calls.recoveryLoaded();
  return { createConfigRecoveryPort: () => ({ list: calls.listRecoveries }) };
});

it("loads each deferred capability on first use and preserves arguments, failures and cancellation", async () => {
  const ports = createTauriFeaturePorts();
  expect(calls.projectsLoaded).not.toHaveBeenCalled();
  expect(calls.modelsLoaded).not.toHaveBeenCalled();
  expect(calls.recoveryLoaded).not.toHaveBeenCalled();
  expect(calls.kitsLoaded).not.toHaveBeenCalled();
  expect(calls.verificationLoaded).not.toHaveBeenCalled();

  await expect(ports.projects.list()).resolves.toEqual([]);
  await expect(ports.projects.list()).resolves.toEqual([]);
  expect(calls.projectsLoaded).toHaveBeenCalledOnce();
  expect(calls.listProjects).toHaveBeenCalledTimes(2);
  expect(calls.kitsLoaded).not.toHaveBeenCalled();
  expect(calls.verificationLoaded).not.toHaveBeenCalled();

  const identity = {
    kitId: "weekly",
    kitVersion: "1.0.0",
    manifestDigest: "a".repeat(64),
  };
  await expect(ports.deliveryKits.previewBuiltin(identity)).resolves.toEqual({
    previewId: "fixture",
  });
  expect(calls.kitsLoaded).toHaveBeenCalledOnce();
  expect(calls.previewKit).toHaveBeenCalledWith(identity);
  expect(calls.verificationLoaded).not.toHaveBeenCalled();
  const failure = new Error("controlled adapter failure");
  calls.previewKit.mockRejectedValueOnce(failure);
  await expect(ports.deliveryKits.previewBuiltin(identity)).rejects.toBe(
    failure,
  );

  await expect(ports.verification.cancel("project-fixture")).resolves.toBe(
    true,
  );
  expect(calls.cancelVerification).toHaveBeenCalledWith("project-fixture");
  await expect(
    ports.verification.export("project-fixture", "json"),
  ).resolves.toBe(false);
  expect(calls.exportHandoff).toHaveBeenCalledWith("project-fixture", "json");
  expect(calls.verificationLoaded).toHaveBeenCalledOnce();
  expect(calls.modelsLoaded).not.toHaveBeenCalled();
  expect(calls.recoveryLoaded).not.toHaveBeenCalled();
  await expect(ports.providers.getSummary("codex")).resolves.toEqual([]);
  expect(calls.modelsLoaded).toHaveBeenCalledOnce();
  expect(calls.providerSummary).toHaveBeenCalledWith("codex");
  await expect(ports.configRecovery.list([])).resolves.toEqual([]);
  expect(calls.recoveryLoaded).toHaveBeenCalledOnce();
  expect(calls.listRecoveries).toHaveBeenCalledWith([]);
});
