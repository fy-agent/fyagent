import { invoke } from "@tauri-apps/api/core";
import {
  bindKitSchema,
  credentialBindingSchema,
  credentialsSchema,
  customerSchema,
  customersSchema,
  dependencySnapshotSchema,
  mutationSchema,
  projectContextSchema,
  projectIdSchema,
  projectSchema,
  projectsSchema,
  resourceKindSchema,
  resourcesSchema,
} from "../../../../domain/projects";
import type { ProjectsPort } from "../../../features/projects";

export function createProjectsPort(): ProjectsPort {
  return {
    listCustomers: async () =>
      customersSchema.parse(await invoke<unknown>("projects_list_customers")),
    createCustomer: async (name) =>
      customerSchema.parse(
        await invoke<unknown>("projects_create_customer", { name }),
      ),
    updateCustomer: async (customerId, expectedRevision, name, archived) =>
      customerSchema.parse(
        await invoke<unknown>("projects_update_customer", {
          customerId: projectIdSchema.parse(customerId),
          expectedRevision,
          name,
          archived,
        }),
      ),
    list: async () =>
      projectsSchema.parse(await invoke<unknown>("projects_list")),
    get: async (projectId) =>
      projectSchema.parse(
        await invoke<unknown>("projects_get", {
          projectId: projectIdSchema.parse(projectId),
        }),
      ),
    create: async (customerId, name) =>
      projectSchema.parse(
        await invoke<unknown>("projects_create", {
          customerId: projectIdSchema.parse(customerId),
          name,
        }),
      ),
    update: async (request, name, archived) =>
      projectSchema.parse(
        await invoke<unknown>("projects_update", {
          request: mutationSchema.parse(request),
          name,
          archived,
        }),
      ),
    resourceOptions: async () =>
      resourcesSchema.parse(await invoke<unknown>("projects_resource_options")),
    credentialOptions: async () =>
      credentialsSchema.parse(
        await invoke<unknown>("projects_credential_options"),
      ),
    bindResource: async (request, resource, model) =>
      projectSchema.parse(
        await invoke<unknown>("projects_bind_resource", {
          request: mutationSchema.parse(request),
          kind: resourceKindSchema.parse(resource.kind),
          agentId: resource.agentId,
          rawId: resource.rawId,
          model,
        }),
      ),
    removeResource: async (request, resource) =>
      projectSchema.parse(
        await invoke<unknown>("projects_remove_resource", {
          request: mutationSchema.parse(request),
          kind: resourceKindSchema.parse(resource.kind),
          agentId: resource.agentId,
          rawId: resource.rawId,
        }),
      ),
    bindCredential: async (request, credential) => {
      const c = credentialBindingSchema.parse({
        credentialId: credential.credentialId,
        purpose: credential.purpose,
        consumer: credential.consumer,
        pinnedGeneration: 0,
      });
      return projectSchema.parse(
        await invoke<unknown>("projects_bind_credential", {
          request: mutationSchema.parse(request),
          credentialId: c.credentialId,
          purpose: c.purpose,
          consumer: c.consumer,
        }),
      );
    },
    removeCredential: async (request, credentialId) =>
      projectSchema.parse(
        await invoke<unknown>("projects_remove_credential", {
          request: mutationSchema.parse(request),
          credentialId,
        }),
      ),
    getContext: async (projectId) =>
      projectContextSchema.parse(
        await invoke<unknown>("projects_get_context", {
          projectId: projectIdSchema.parse(projectId),
        }),
      ),
    writeContext: async (request, content) =>
      projectContextSchema.parse(
        await invoke<unknown>("projects_write_context", {
          request: mutationSchema.parse(request),
          content,
        }),
      ),
    prepareCodex: async (request) =>
      projectContextSchema.parse(
        await invoke<unknown>("projects_prepare_codex", {
          request: mutationSchema.parse(request),
        }),
      ),
    bindDeliveryKit: async (request) =>
      projectSchema.parse(
        await invoke<unknown>("projects_bind_delivery_kit", {
          request: bindKitSchema.parse(request),
        }),
      ),
    dependencySnapshot: async (projectId) =>
      dependencySnapshotSchema.parse(
        await invoke<unknown>("projects_dependency_snapshot", {
          projectId: projectIdSchema.parse(projectId),
        }),
      ),
  };
}
