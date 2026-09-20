import * as z from "zod/mini";

export const CONFIG_PACK_MAX_BYTES = 64 * 1024;
export const CONFIG_PACK_MAX_ENTRIES = 32;
const sensitive =
  /sk[-_]|bearer |secret_?ref|credentialref|api_?key|access_token|refresh_token|password|-----begin|eyj/iu;
const bytes = (s: string) => new TextEncoder().encode(s).length;
const name = z
  .string()
  .check(
    z.refine(
      (s) =>
        s.length > 0 &&
        bytes(s) <= 160 &&
        s.trim() === s &&
        /^[\p{L}\p{N} ._()-]+$/u.test(s) &&
        !s.includes("..") &&
        !sensitive.test(s),
    ),
  );
const model = z
  .string()
  .check(
    z.refine(
      (s) =>
        s.length > 0 &&
        s.length <= 128 &&
        /^[a-z\d._:/-]+$/iu.test(s) &&
        !s.startsWith("/") &&
        !s.split("/").some((part) => part === ".") &&
        !s.includes("..") &&
        !s.includes(":/") &&
        !sensitive.test(s),
    ),
  );
const endpoint = z.string().check(
  z.refine((s) => {
    if (
      bytes(s) > 2048 ||
      /[%\\\s$~]/u.test(s) ||
      sensitive.test(s) ||
      s.split("/").some((p) => p === "." || p === "..")
    )
      return false;
    try {
      const url = new URL(s);
      return (
        ["https:", "http:"].includes(url.protocol) &&
        !!url.hostname &&
        !url.username &&
        !url.password &&
        !url.search &&
        !url.hash
      );
    } catch {
      return false;
    }
  }),
);
export const portableProviderSchema = z
  .strictObject({
    app: z.enum(["claude", "codex"]),
    name,
    endpoint,
    model,
    wireApi: z.nullable(z.enum(["responses", "chat"])),
  })
  .check(z.refine((p) => (p.app === "codex") === (p.wireApi !== null)));
export type PortableProvider = z.infer<typeof portableProviderSchema>;
const providers = z
  .array(portableProviderSchema)
  .check(z.maxLength(CONFIG_PACK_MAX_ENTRIES));
const manifestSchema = z.strictObject({
  format: z.literal("fyagent-config-pack/v1"),
  providers: providers.check(
    z.minLength(1),
    z.refine(
      (items) =>
        new Set(items.map((p) => JSON.stringify([p.app, p.name]))).size ===
        items.length,
    ),
  ),
});
export type ConfigPack = z.infer<typeof manifestSchema>;
const digest = z.string().check(z.regex(/^[a-f0-9]{64}$/u));
const id = z
  .string()
  .check(
    z.regex(
      /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/u,
    ),
  );
const action = z.enum(["add", "skip", "rename", "overwrite"]);
const choiceSchema = z.strictObject({ action, name: z.nullable(name) });
export type ImportChoice = z.infer<typeof choiceSchema>;
const entrySchema = z
  .strictObject({
    provider: portableProviderSchema,
    action,
    conflict: z.boolean(),
    canOverwrite: z.boolean(),
    existing: z.nullable(portableProviderSchema),
    credentialsRequired: z.literal(true),
  })
  .check(
    z.refine(
      (e) =>
        (!e.canOverwrite || (e.conflict && e.existing !== null)) &&
        (e.action !== "overwrite" || e.canOverwrite) &&
        (e.action !== "add" || !e.conflict),
    ),
  );
const previewSchema = z.strictObject({
  previewId: id,
  digest,
  entries: z
    .array(entrySchema)
    .check(z.minLength(1), z.maxLength(CONFIG_PACK_MAX_ENTRIES)),
});
const exportSchema = z.strictObject({
  exportId: id,
  digest,
  text: z.string().check(z.maxLength(CONFIG_PACK_MAX_BYTES)),
});
const candidatesSchema = z.strictObject({
  entries: z
    .array(
      z.strictObject({ selectionId: digest, provider: portableProviderSchema }),
    )
    .check(z.maxLength(256)),
  excluded: z.number().check(z.int(), z.minimum(0), z.maximum(256)),
});
const resultSchema = z.strictObject({
  providers,
  skipped: z
    .number()
    .check(z.int(), z.minimum(0), z.maximum(CONFIG_PACK_MAX_ENTRIES)),
});
export type ImportPreview = z.infer<typeof previewSchema>;
export type ExportPreview = z.infer<typeof exportSchema>;
export type PackCandidates = z.infer<typeof candidatesSchema>;
export type ImportResult = z.infer<typeof resultSchema>;

export function parseConfigPack(text: string): ConfigPack {
  if (bytes(text) > CONFIG_PACK_MAX_BYTES) throw new Error("too_large");
  return z.parse(manifestSchema, JSON.parse(text) as unknown);
}
export const parsePackCandidates = (value: unknown): PackCandidates =>
  z.parse(candidatesSchema, value);
export const parseImportPreview = (value: unknown): ImportPreview =>
  z.parse(previewSchema, value);
export function parseExportPreview(value: unknown): ExportPreview {
  const preview = z.parse(exportSchema, value);
  parseConfigPack(preview.text);
  return preview;
}
export function parseImportResult(
  value: unknown,
  preview: ImportPreview,
): ImportResult {
  const result = z.parse(resultSchema, value);
  const expected = preview.entries
    .filter((e) => e.action !== "skip")
    .map((e) => e.provider);
  if (
    JSON.stringify(result.providers) !== JSON.stringify(expected) ||
    result.skipped !== preview.entries.length - expected.length
  )
    throw new Error("readback_failed");
  return result;
}
export function importRequest(text: string, choices: ImportChoice[]) {
  parseConfigPack(text);
  const parsed = z.parse(
    z.array(choiceSchema).check(z.maxLength(CONFIG_PACK_MAX_ENTRIES)),
    choices,
  );
  return { text, choices: parsed };
}
export function confirmRequest(previewId: string, expectedDigest: string) {
  return z.parse(z.strictObject({ previewId: id, digest }), {
    previewId,
    digest: expectedDigest,
  });
}
export const parseSelection = (value: string[]) =>
  z.parse(
    z.array(digest).check(
      z.minLength(1),
      z.maxLength(CONFIG_PACK_MAX_ENTRIES),
      z.refine((s) => new Set(s).size === s.length),
    ),
    value,
  );
