import type {
  KitDemoResult,
  KitIdentity,
  KitPreview,
  KitView,
} from "../../domain/delivery-kits";

export interface DeliveryKitsPort {
  list(): Promise<KitView[]>;
  previewBuiltin(identity: KitIdentity): Promise<KitPreview>;
  pickImport(): Promise<KitPreview | null>;
  apply(preview: KitPreview): Promise<KitView>;
  cancel(preview: KitPreview): Promise<void>;
  previewExport(identity: KitIdentity): Promise<KitPreview>;
  saveExport(preview: KitPreview): Promise<boolean>;
  runDemo(identity: KitIdentity): Promise<KitDemoResult>;
}
export interface KitProjectContext {
  projectId: string;
  projectRevision: number;
}
export interface KitProjectAdapter {
  bindDeliveryKit(
    request: KitIdentity & {
      projectId: string;
      expectedRevision: number;
      bindingIntentId: string;
    },
  ): Promise<{ projectId: string; projectRevision: number }>;
}
// Root supplies an adapter to a native command that reads project dependencies and re-runs
// the validator itself. Renderer results are never passed back as evidence authority.
export interface KitEvidenceAdapter {
  recordLocalFixture(
    request: KitIdentity & KitProjectContext,
  ): Promise<{ recordId: string }>;
}
export const kitErrorMessages = {
  invalid_package: "交付包格式或内容不完整，请检查文件。",
  unsupported_schema: "暂不支持这个交付包格式。",
  incompatible_host: "当前版本缺少这个包所需的能力。",
  unsafe_content: "交付包包含不允许的内容，无法导入或分享。",
  content_conflict:
    "已有同版本的不同内容，或保存位置已存在文件，请选择新版本或新文件名。",
  not_found: "找不到交付包，请刷新。",
  preview_expired: "预览已失效，请重新打开。",
  invalid_preview: "预览内容已变化，请重新打开。",
  library_unavailable: "无法读取交付包，请检查文件访问权限。",
  write_failed: "未能保存交付包，请检查保存位置后重试。",
  readback_failed: "无法确认交付包完整性，请检查文件后重试。",
  export_not_allowed: "这个包未经分享审查，暂不能导出。",
  unsupported_validator: "这个入门包需按说明人工检查。",
  busy: "交付包正在处理，请稍后重试。",
  dependency_unavailable: "项目或验证记录暂不可用。",
} as const;
export class DeliveryKitError extends Error {
  constructor(readonly code: keyof typeof kitErrorMessages) {
    super(kitErrorMessages[code]);
    this.name = "DeliveryKitError";
  }
}
export function safeKitError(error: unknown): DeliveryKitError {
  if (error instanceof DeliveryKitError) return error;
  if (
    typeof error === "string" &&
    Object.prototype.hasOwnProperty.call(kitErrorMessages, error)
  )
    return new DeliveryKitError(error as keyof typeof kitErrorMessages);
  return new DeliveryKitError("library_unavailable");
}
