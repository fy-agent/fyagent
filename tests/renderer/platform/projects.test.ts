import { describe, expect, it, vi } from "vitest";
import { createProjectsPort } from "@/shared/platform/tauri/feature-ports/projects";
import { projectSchema, dependencySnapshotSchema } from "@/domain/projects";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const project = {
  projectId: id,
  customerId: id,
  name: "P",
  projectRevision: 1,
  archived: false,
  resources: [],
  credentials: [],
  kit: null,
  contextGeneration: null,
  codexPrepared: false,
  createdAt: "2026-09-19T00:00:00Z",
  updatedAt: "2026-09-19T00:00:00Z",
};
describe("projects wire boundary", () => {
  it("rejects secret/excess payloads, invalid IDs and unsafe revisions", () => {
    expect(() =>
      projectSchema.parse({ ...project, secretRef: "sec_not_public" }),
    ).toThrow();
    expect(() =>
      projectSchema.parse({ ...project, projectId: "../../escape" }),
    ).toThrow();
    expect(() =>
      projectSchema.parse({ ...project, projectRevision: -1 }),
    ).toThrow();
    expect(() =>
      projectSchema.parse({
        ...project,
        projectRevision: Number.MAX_SAFE_INTEGER + 1,
      }),
    ).toThrow();
    expect(() =>
      dependencySnapshotSchema.parse({ projectId: id, state: "success" }),
    ).toThrow();
  });
  it("sends only identity, CAS and native kit binding intent", async () => {
    invoke.mockResolvedValueOnce(project);
    const request = {
      projectId: id,
      expectedRevision: 0,
      kitId: "weekly",
      kitVersion: "1.0.0",
      manifestDigest: "a".repeat(64),
      bindingIntentId: id,
    };
    expect(await createProjectsPort().bindDeliveryKit(request)).toEqual(
      project,
    );
    expect(invoke).toHaveBeenCalledWith("projects_bind_delivery_kit", {
      request,
    });
  });
  it("parses native observations instead of accepting a claimed success", async () => {
    invoke.mockResolvedValueOnce({ ...project, rawSecret: "not allowed" });
    await expect(createProjectsPort().get(id)).rejects.toThrow();
  });
});
