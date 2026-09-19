import * as z from "zod/mini";

export const VERIFICATION_STAGES = [
  "configuration_saved",
  "authentication_available",
  "tool_callable",
  "sample_passed",
  "customer_accepted",
] as const;
export const CHECKERS = [
  "saved_configuration_readback",
  "saved_model_probe",
  "kit_validator",
] as const;
export const KIT_FIXTURES = ["baseline", "missing_field"] as const;
export const ROLLBACKS = [
  "review_configuration",
  "guarded_file_recovery",
  "manual_only",
  "not_available",
] as const;
export const REASONS = [
  "configuration_not_saved",
  "check_cancelled",
  "manual_record",
  "saved_projection_confirmed",
  "saved_projection_different",
  "saved_projection_unavailable",
  "saved_model_unavailable",
  "credential_generation_unavailable",
  "model_identity_confirmed",
  "model_identity_unconfirmed",
  "model_request_failed",
  "local_sample_checked",
  "kit_validator_unavailable",
  "dependencies_changed",
  "revoked",
  "dependency_unverifiable",
] as const;
export const projectIdSchema = z
  .string()
  .check(
    z.regex(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u),
  );
const revision = z
  .number()
  .check(z.int(), z.minimum(0), z.maximum(Number.MAX_SAFE_INTEGER));
const timestamp = z.string().check(
  z.iso.datetime(),
  z.refine((v) => Number.isFinite(Date.parse(v))),
);
const label = z.string().check(
  z.minLength(1),
  z.maxLength(160),
  z.regex(/^[\p{L}\p{N} _（）()，、-]+$/u),
  z.refine(
    (v) =>
      v.trim() === v &&
      !/secretref|sk-|bearer|api_key|apikey|password|token/iu.test(v),
  ),
);
const bounded = z
  .string()
  .check(z.minLength(1), z.maxLength(128), z.regex(/^[a-zA-Z0-9._+-]+$/u));
const externalReference = z.string().check(
  z.refine((value) => {
    if (label.safeParse(value).success) return true;
    if (
      value.length > 2048 ||
      value.trim() !== value ||
      Array.from(value).some(
        (c) =>
          c.charCodeAt(0) < 32 ||
          c.charCodeAt(0) === 127 ||
          /\s/u.test(c) ||
          ["\\", "%", "<", ">", '"', "[", "]"].includes(c),
      ) ||
      /secretref|sk-|bearer|api_key|apikey|password|token/iu.test(value)
    )
      return false;
    try {
      const url = new URL(value);
      return (
        value.startsWith("https://") &&
        url.protocol === "https:" &&
        url.hostname.includes(".") &&
        url.username === "" &&
        url.password === "" &&
        !value.includes("?") &&
        !value.includes("#")
      );
    } catch {
      return false;
    }
  }),
);
const externalBasis = z.strictObject({
  reference: externalReference,
  issuer: label,
});
const manual = z.strictObject({
  person: label,
  role: label,
  scope: label,
  externalBasis: z.nullable(externalBasis),
});
const ids = z.array(projectIdSchema).check(z.maxLength(32));
export const kitSchema = z.strictObject({
  kitId: bounded,
  kitVersion: bounded,
  manifestDigest: z.string().check(z.regex(/^[a-f0-9]{64}$/u)),
});
const safeInteger = z
  .number()
  .check(
    z.int(),
    z.minimum(Number.MIN_SAFE_INTEGER),
    z.maximum(Number.MAX_SAFE_INTEGER),
  );
export const evidenceSchema = z
  .strictObject({
    id: projectIdSchema,
    projectRevision: revision,
    kit: z.nullable(kitSchema),
    stage: z.enum(VERIFICATION_STAGES),
    outcome: z.enum([
      "passed",
      "failed",
      "unknown",
      "unsupported",
      "cancelled",
    ]),
    validity: z.enum(["current", "stale", "revoked", "unverifiable"]),
    sourceClass: z.enum([
      "native_local",
      "native_remote",
      "local_fixture",
      "manual_record",
    ]),
    checkerId: z.enum([...CHECKERS, "external_manual_record"]),
    fixture: z.nullable(z.enum(KIT_FIXTURES)),
    sample: z.nullable(
      z
        .strictObject({
          inputDigest: z.string().check(z.regex(/^[a-f0-9]{64}$/u)),
          code: z.enum([
            "ok",
            "invalid_input",
            "duplicate_row",
            "period_mismatch",
            "currency_mismatch",
            "zero_previous",
            "missing_period",
            "invalid_amount",
          ]),
          matchesExpectation: z.boolean(),
          validator: z.literal("weekly-report/v1"),
          metrics: z.nullable(
            z.strictObject({
              currentMinor: safeInteger,
              previousMinor: safeInteger,
              growthBps: safeInteger,
              targetBps: safeInteger,
            }),
          ),
          sourceRowIds: z
            .array(
              z.enum(["row-1", "row-2", "row-3", "row-4", "row-5", "row-6"]),
            )
            .check(
              z.maxLength(6),
              z.refine((ids) => new Set(ids).size === ids.length),
            ),
        })
        .check(z.refine((s) => (s.code === "ok") === (s.metrics !== null))),
    ),
    checkerVersion: revision,
    appVersion: bounded,
    observedAt: timestamp,
    recordedAt: timestamp,
    expiresAt: z.nullable(timestamp),
    reasonCode: z.enum(REASONS),
    basisEvidenceIds: ids,
    manual: z.nullable(manual),
  })
  .check(
    z.refine(
      (e) => (e.sourceClass === "manual_record") === (e.manual !== null),
    ),
    z.refine(
      (e) =>
        e.stage !== "customer_accepted" || e.sourceClass === "manual_record",
    ),
  );
export const handoffSchema = z.strictObject({
  items: z
    .array(
      z.strictObject({
        title: label,
        owner: z.nullable(label),
        completed: z.boolean(),
      }),
    )
    .check(z.maxLength(100)),
  rollback: z.nullable(z.enum(ROLLBACKS)),
});
export const snapshotSchema = z
  .strictObject({
    schemaVersion: z.literal(1),
    projectId: projectIdSchema,
    projectRevision: z.nullable(revision),
    available: z.boolean(),
    checkedAt: timestamp,
    evidence: z.array(evidenceSchema).check(z.maxLength(2000)),
    handoff: handoffSchema,
    handoffRevision: revision,
  })
  .check(
    z.refine((s) => s.available === (s.projectRevision !== null)),
    z.refine(
      (s) => new Set(s.evidence.map((e) => e.id)).size === s.evidence.length,
    ),
  );
export const runSchema = z
  .strictObject({
    projectId: projectIdSchema,
    expectedRevision: revision,
    checker: z.enum(CHECKERS),
    runId: projectIdSchema,
    fixture: z.nullable(z.enum(KIT_FIXTURES)),
  })
  .check(
    z.refine((r) => (r.checker === "kit_validator") === (r.fixture !== null)),
  );
export const manualSchema = z
  .strictObject({
    projectId: projectIdSchema,
    expectedRevision: revision,
    stage: z.enum(VERIFICATION_STAGES),
    outcome: z.enum([
      "passed",
      "failed",
      "unknown",
      "unsupported",
      "cancelled",
    ]),
    person: label,
    role: label,
    scope: label,
    observedAt: timestamp,
    externalBasis: z.nullable(externalBasis),
    basisEvidenceIds: ids,
  })
  .check(
    z.refine((r) => r.externalBasis !== null || r.basisEvidenceIds.length > 0),
  );
export const revokeSchema = z.strictObject({
  projectId: projectIdSchema,
  evidenceId: projectIdSchema,
});
export const saveHandoffSchema = z.strictObject({
  projectId: projectIdSchema,
  expectedRevision: revision,
  handoffRevision: revision,
  handoff: handoffSchema,
});
export const previewSchema = z.strictObject({
  snapshot: snapshotSchema,
  json: z.string().check(z.maxLength(4_000_000)),
  markdown: z.string().check(z.maxLength(4_000_000)),
});

export type Stage = z.infer<typeof evidenceSchema>["stage"];
export type Evidence = z.infer<typeof evidenceSchema>;
export type VerificationSnapshot = z.infer<typeof snapshotSchema>;
export type ManualRequest = z.infer<typeof manualSchema>;
export type RunRequest = z.infer<typeof runSchema>;
export type RevokeRequest = z.infer<typeof revokeSchema>;
export type SaveHandoffRequest = z.infer<typeof saveHandoffSchema>;
export type HandoffNotes = z.infer<typeof handoffSchema>;
export type HandoffPreview = z.infer<typeof previewSchema>;

export interface VerificationPort {
  cancel(projectId: string): Promise<boolean>;
  get(projectId: string): Promise<VerificationSnapshot>;
  run(request: RunRequest): Promise<VerificationSnapshot>;
  record(request: ManualRequest): Promise<VerificationSnapshot>;
  revoke(request: RevokeRequest): Promise<VerificationSnapshot>;
  saveHandoff(request: SaveHandoffRequest): Promise<VerificationSnapshot>;
  preview(projectId: string): Promise<HandoffPreview>;
  export(projectId: string, format: "json" | "markdown"): Promise<boolean>;
}

export const STAGE_LABELS: Record<Stage, string> = {
  configuration_saved: "配置已保存",
  authentication_available: "认证可用",
  tool_callable: "工具可调用",
  sample_passed: "样本通过",
  customer_accepted: "客户已验收",
};
export const SOURCE_LABELS: Record<Evidence["sourceClass"], string> = {
  native_local: "本机检查",
  native_remote: "服务请求",
  local_fixture: "本机模拟样本",
  manual_record: "人工登记",
};
export const REASON_LABELS: Record<Evidence["reasonCode"], string> = {
  check_cancelled: "已取消检查",
  configuration_not_saved: "配置尚未保存",
  manual_record: "人工登记的检查或验收结果",
  saved_projection_confirmed: "已保存，目标配置一致",
  saved_projection_different: "已保存，目标配置有差异",
  saved_projection_unavailable: "已保存，目标是否生效尚未确认",
  saved_model_unavailable: "此模型尚不能检查",
  credential_generation_unavailable: "无法确认凭据是否变更，请重新检查账号设置",
  model_identity_confirmed: "指定模型已响应",
  model_identity_unconfirmed: "无法确认响应模型",
  model_request_failed: "请求失败，请检查服务和账号设置",
  local_sample_checked: "本机样本检查完成，不代表外部服务可用",
  kit_validator_unavailable: "此交付包尚不能检查",
  dependencies_changed: "配置、范围或依据已变化，请重新检查",
  revoked: "已撤销",
  dependency_unverifiable: "无法确认当前配置，请刷新后检查",
};
export function evidenceLabel(e: Evidence, now = Date.now()): string {
  if (e.validity === "revoked") return "已撤销";
  if (
    e.validity !== "current" ||
    Date.parse(e.recordedAt) > now ||
    (e.expiresAt !== null && Date.parse(e.expiresAt) <= now)
  )
    return "待复核";
  return {
    passed: "通过",
    failed: "失败",
    unknown: "未确认",
    unsupported: "暂不支持",
    cancelled: "已取消",
  }[e.outcome];
}

export const CHECKER_LABELS: Record<Evidence["checkerId"], string> = {
  saved_configuration_readback: "保存配置检查",
  saved_model_probe: "已保存模型检查",
  kit_validator: "本机样本检查",
  external_manual_record: "人工登记",
};

export const SAMPLE_CODE_LABELS: Record<
  NonNullable<Evidence["sample"]>["code"],
  string
> = {
  ok: "业务输入通过",
  invalid_input: "缺少必填字段或输入格式有误",
  duplicate_row: "存在重复数据行",
  period_mismatch: "数据期间不一致",
  currency_mismatch: "金额币种不一致",
  zero_previous: "上期金额为零，无法计算增长率",
  missing_period: "缺少本期或上期数据",
  invalid_amount: "金额或目标值无效",
};
