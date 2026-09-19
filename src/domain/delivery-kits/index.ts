import * as z from "zod/mini";

const id = z.string().check(z.regex(/^[a-z][a-z0-9-]{0,63}$/u));
const digest = z.string().check(z.regex(/^[a-f0-9]{64}$/u));
const version = z
  .string()
  .check(
    z.maxLength(32),
    z.regex(/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/u),
  );
const text = z.string().check(z.maxLength(262144));
const ids = z.array(id).check(z.maxLength(100));
const contentRef = z.strictObject({ id, resourceId: id, version });
export const kitIdentitySchema = z.strictObject({
  kitId: id,
  kitVersion: version,
  manifestDigest: digest,
});
const manifestSchema = z.strictObject({
  schemaVersion: z.literal("fyagent-delivery-kit/v1"),
  id,
  version,
  title: z.string().check(z.minLength(1), z.maxLength(100)),
  summary: z.string().check(z.maxLength(1000)),
  scenario: z.enum(["weekly_report", "knowledge_support", "business_query"]),
  compatibility: z.strictObject({
    minFyAgentVersion: version,
    requiredCapabilities: z.array(text).check(z.maxLength(100)),
  }),
  provenance: z.strictObject({
    publisherLabel: text,
    license: text,
    sourcePresetIds: ids,
    sourceRecipeIds: ids,
  }),
  inputs: z
    .array(
      z.strictObject({
        id,
        label: text,
        description: text,
        required: z.boolean(),
      }),
    )
    .check(z.maxLength(100)),
  connections: z
    .array(
      z.strictObject({
        id,
        recipeId: id,
        purpose: text,
        permissionIds: ids,
        credentialSlotIds: ids,
        optional: z.boolean(),
      }),
    )
    .check(z.maxLength(100)),
  credentialSlots: z
    .array(z.strictObject({ id, purpose: text, required: z.boolean() }))
    .check(z.maxLength(100)),
  permissions: z
    .array(
      z.strictObject({
        id,
        resourceClass: z.enum([
          "business_table",
          "knowledge_documents",
          "database",
        ]),
        access: z.literal("read"),
        rationale: text,
      }),
    )
    .check(z.maxLength(100)),
  prompts: z.array(contentRef).check(z.minLength(1), z.maxLength(100)),
  skills: z.array(contentRef).check(z.minLength(1), z.maxLength(100)),
  resources: z
    .array(
      z.strictObject({
        id,
        mediaType: z.enum(["text/markdown", "application/json"]),
        text,
        sha256: digest,
      }),
    )
    .check(z.minLength(1), z.maxLength(100)),
  fixtures: z
    .array(
      z.strictObject({
        id,
        inputResourceId: id,
        expectedResourceId: id,
        kind: z.enum(["positive", "negative"]),
        provenance: z.literal("synthetic"),
      }),
    )
    .check(z.minLength(1), z.maxLength(100)),
  checks: z
    .array(
      z.strictObject({
        id,
        validator: z.enum(["weekly_report_v1", "manual_review"]),
        fixtureIds: ids,
        description: text,
      }),
    )
    .check(z.minLength(1), z.maxLength(100)),
  handoff: id,
  rollback: id,
});
const viewSchema = z
  .strictObject({
    identity: kitIdentitySchema,
    manifest: manifestSchema,
    installed: z.boolean(),
    builtin: z.boolean(),
    compatible: z.boolean(),
    exportable: z.boolean(),
    connectionsChecked: z.literal(false),
  })
  .check(
    z.refine(
      (v) =>
        v.identity.kitId === v.manifest.id &&
        v.identity.kitVersion === v.manifest.version &&
        (!v.exportable || v.builtin),
    ),
  );
const token = z
  .string()
  .check(
    z.regex(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u,
    ),
  );
const previewSchema = z.strictObject({
  previewId: token,
  kind: z.enum(["import", "export"]),
  kit: viewSchema,
  conflict: z.boolean(),
});
const amount = z.number().check(z.refine(Number.isSafeInteger));
const metrics = z.strictObject({
  currentMinor: amount,
  previousMinor: amount,
  growthBps: amount,
  targetBps: amount,
});
export const businessCodes = [
  "ok",
  "invalid_input",
  "duplicate_row",
  "period_mismatch",
  "currency_mismatch",
  "zero_previous",
  "missing_period",
  "invalid_amount",
] as const;
const caseSchema = z
  .strictObject({
    fixtureId: id,
    inputDigest: digest,
    code: z.enum(businessCodes),
    metrics: z.nullable(metrics),
    sourceRowIds: z.array(id).check(z.maxLength(1000)),
    matchesExpectation: z.boolean(),
  })
  .check(z.refine((v) => (v.code === "ok") === (v.metrics !== null)));
const demoSchema = z.strictObject({
  kitId: id,
  kitVersion: version,
  manifestDigest: digest,
  sourceClass: z.literal("local_fixture"),
  validatorVersion: z.literal("weekly-report/v1"),
  checkedAt: z.string().check(z.iso.datetime({ offset: true })),
  cases: z.array(caseSchema).check(z.minLength(1), z.maxLength(100)),
});

export type KitIdentity = z.infer<typeof kitIdentitySchema>;
export type KitView = z.infer<typeof viewSchema>;
export type KitPreview = z.infer<typeof previewSchema>;
export type KitDemoResult = z.infer<typeof demoSchema>;
export type BusinessCode = (typeof businessCodes)[number];

export function parseKitIdentity(v: unknown): KitIdentity {
  const r = kitIdentitySchema.safeParse(v);
  if (!r.success) throw new Error("交付包标识无效");
  return r.data;
}
export function parseKitView(v: unknown): KitView {
  const r = viewSchema.safeParse(v);
  if (!r.success) throw new Error("无法读取交付包，请刷新后重试");
  return r.data;
}
export function parseKitList(v: unknown): KitView[] {
  const r = z.array(viewSchema).check(z.maxLength(1000)).safeParse(v);
  if (!r.success) throw new Error("无法读取交付包目录");
  return r.data;
}
export function parseKitPreview(v: unknown): KitPreview {
  const r = previewSchema.safeParse(v);
  if (!r.success) throw new Error("无法读取交付包预览");
  return r.data;
}
export function parseKitDemo(v: unknown, expected: KitIdentity): KitDemoResult {
  const r = demoSchema.safeParse(v);
  if (
    !r.success ||
    r.data.kitId !== expected.kitId ||
    r.data.kitVersion !== expected.kitVersion ||
    r.data.manifestDigest !== expected.manifestDigest
  )
    throw new Error("无法确认样例检查结果，请重新运行");
  return r.data;
}
export function assertPreviewRequest(
  previewId: unknown,
  manifestDigest: unknown,
): { previewId: string; manifestDigest: string } {
  const r = z
    .strictObject({ previewId: token, manifestDigest: digest })
    .safeParse({ previewId, manifestDigest });
  if (!r.success) throw new Error("交付包预览已失效，请重新打开");
  return r.data;
}

export const businessLabels: Record<BusinessCode, string> = {
  ok: "指标核对完成",
  invalid_input: "输入缺少字段或格式不正确",
  duplicate_row: "来源行重复，已停止计算",
  period_mismatch: "存在所选期间以外的数据",
  currency_mismatch: "币种不一致，已停止计算",
  zero_previous: "上期收入为零，不能计算增长率",
  missing_period: "缺少一个期间的数据",
  invalid_amount: "金额或目标超出允许范围",
};
