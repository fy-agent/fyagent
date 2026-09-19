import * as z from "zod/mini";

const id = z
  .string()
  .check(
    z.regex(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u,
    ),
  );
const revision = z
  .number()
  .check(z.int(), z.minimum(0), z.maximum(Number.MAX_SAFE_INTEGER - 1));
const text = z.string();
export const resourceKindSchema = z.enum([
  "provider",
  "mcp",
  "skill",
  "prompt",
  "memory",
]);
export const observationSchema = z.enum([
  "unverifiable",
  "matched",
  "drifted",
  "missing",
  "unavailable",
  "revoked",
  "purpose_mismatch",
]);
export const customerSchema = z.strictObject({
  customerId: id,
  name: text,
  revision,
  archived: z.boolean(),
});
export const resourceBindingSchema = z.strictObject({
  kind: resourceKindSchema,
  agentId: text,
  rawId: text,
  model: z.nullable(text),
  pinnedVersion: z.nullable(text),
});
export const credentialBindingSchema = z.strictObject({
  credentialId: text,
  purpose: text,
  consumer: text,
  pinnedGeneration: revision,
});
export const kitBindingSchema = z.strictObject({
  kitId: text,
  kitVersion: text,
  manifestDigest: z.string().check(z.regex(/^[0-9a-f]{64}$/u)),
});
export const projectSchema = z.strictObject({
  projectId: id,
  customerId: id,
  name: text,
  projectRevision: revision,
  archived: z.boolean(),
  resources: z.array(resourceBindingSchema),
  credentials: z.array(credentialBindingSchema),
  kit: z.nullable(kitBindingSchema),
  contextGeneration: z.nullable(id),
  codexPrepared: z.boolean(),
  createdAt: text,
  updatedAt: text,
});
const contextState = z.enum(["not_created", "materialized", "unavailable"]);
export const projectContextSchema = z.strictObject({
  projectId: id,
  projectRevision: revision,
  content: text,
  directory: z.nullable(text),
  state: contextState,
  codexInstructions: z.nullable(text),
});
export const resourceOptionSchema = z.strictObject({
  kind: resourceKindSchema,
  agentId: text,
  rawId: text,
  label: text,
  version: z.nullable(text),
});
export const credentialOptionSchema = z.strictObject({
  credentialId: text,
  label: text,
  purpose: text,
  consumer: text,
  generation: revision,
  state: observationSchema,
});
export const dependencySnapshotSchema = z.strictObject({
  projectId: id,
  projectRevision: revision,
  archived: z.boolean(),
  kit: z.nullable(kitBindingSchema),
  kitState: observationSchema,
  resources: z.array(
    z.strictObject({
      resource: resourceBindingSchema,
      observedVersion: z.nullable(text),
      state: observationSchema,
    }),
  ),
  credentials: z.array(
    z.strictObject({
      binding: credentialBindingSchema,
      observedGeneration: z.nullable(revision),
      state: observationSchema,
    }),
  ),
  contextGeneration: z.nullable(id),
  contextState,
  observedAt: text,
  runtimeAvailable: z.boolean(),
});
export const mutationSchema = z.strictObject({
  projectId: id,
  expectedRevision: revision,
});
export const bindKitSchema = z.strictObject({
  projectId: id,
  expectedRevision: revision,
  kitId: text,
  kitVersion: text,
  manifestDigest: z.string().check(z.regex(/^[0-9a-f]{64}$/u)),
  bindingIntentId: id,
});
export const projectIdSchema = id;
export const customersSchema = z.array(customerSchema);
export const projectsSchema = z.array(projectSchema);
export const resourcesSchema = z.array(resourceOptionSchema);
export const credentialsSchema = z.array(credentialOptionSchema);
export type Customer = z.infer<typeof customerSchema>;
export type Project = z.infer<typeof projectSchema>;
export type ProjectContext = z.infer<typeof projectContextSchema>;
export type ProjectDependencySnapshot = z.infer<
  typeof dependencySnapshotSchema
>;
export type ResourceOption = z.infer<typeof resourceOptionSchema>;
export type CredentialOption = z.infer<typeof credentialOptionSchema>;
export type ResourceKind = z.infer<typeof resourceKindSchema>;
export type ProjectMutation = z.infer<typeof mutationSchema>;
export type BindDeliveryKitRequest = z.infer<typeof bindKitSchema>;
