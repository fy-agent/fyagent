import type {
  ExportPreview,
  ImportChoice,
  ImportPreview,
  ImportResult,
  PackCandidates,
  PortableProvider,
} from "../../domain/config-pack";

/** Populate a local model form only; credentials and apply keep their own flow. */
export type ConfigPackFormFill = (provider: Readonly<PortableProvider>) => void;

export interface ConfigPackPort {
  list(): Promise<PackCandidates>;
  pickFile(): Promise<string | null>;
  previewImport(text: string, choices: ImportChoice[]): Promise<ImportPreview>;
  apply(preview: ImportPreview): Promise<ImportResult>;
  previewExport(selection: string[]): Promise<ExportPreview>;
  saveExport(preview: ExportPreview): Promise<boolean>;
  cancel(previewId: string): Promise<void>;
}

const errors: Record<string, string> = {
  invalid_pack: "配置格式不受支持，请检查文件或文本。",
  too_large: "配置内容超过 64 KB，请减少条目后重试。",
  unsafe_content: "配置包含不可迁移的字段、凭据或本机地址，请检查内容。",
  unsupported_version: "此版本的配置包暂不受支持。",
  conflict: "名称已存在或此配置不能覆盖，请选择跳过或重命名。",
  stale_preview: "配置已变化或预览已过期，请重新预览。",
  invalid_preview: "预览无效，请重新预览。",
  storage_unavailable: "无法读取已保存配置，请稍后重试。",
  write_failed: "保存未完成，请重新预览后重试。",
  readback_failed: "无法确认保存结果，请重新打开并检查已保存配置。",
  file_unavailable: "无法读取所选文件，请重新选择 JSON 文件。",
  busy: "已保存连接或待处理预览过多，请整理后重试。",
};
export function safePackError(error: unknown): Error {
  if (error instanceof Error && Object.values(errors).includes(error.message))
    return new Error(error.message);
  return new Error(
    typeof error === "string" && errors[error]
      ? errors[error]
      : error instanceof Error && error.message === "too_large"
        ? errors.too_large
        : error instanceof Error && error.message === "readback_failed"
          ? errors.readback_failed
          : "无法完成配置迁移，请检查内容并重新预览。",
  );
}
