# Codex 一键配置接入生图与 WebSocket — 技术设计

## 目标

在 v2 模型配置页 Codex 模块接入「启用内置生图扩展」与「启用 WebSocket 传输」两个开关，并让后端快速配置路径真实写入这两项能力；开启生图扩展时，官方 provider 的 `requires_openai_auth` 联动为 `false`。

## 现状梳理

### 后端已具备的能力（`src-tauri/src/codex_config.rs`）

- 生图扩展 header：
  - `CODEX_IMAGE_EXTENSION_HEADER = "x-openai-actor-authorization"`
  - `CODEX_IMAGE_EXTENSION_VALUE = "local-image-extension"`
  - `set_managed_image_header(provider_table, enabled)`：写入/删除 `http_headers` 中的该 header（含大小写冲突修复、非法值修复）。
- WebSocket：`supports_websockets` 字段的读取（`websocket_state`）与写入（`patch_codex_provider_features`）。
- `CodexProviderFeatureIntent { image_extension: Option<bool>, websockets: Option<bool> }`。
- `analyze_codex_provider_features(provider, is_new)` / `patch_codex_provider_features(provider, intent, is_new)`：旧 provider 表单的分析/补丁 API，返回 `CodexProviderFeaturePatchResult { state, toml_text, image_extension_configured, codex_native_capabilities_generated_provider }`。
- `prepare_codex_provider_features_for_save(provider, is_new)`：保存时对非官方 provider 应用默认生图迁移（默认开启生图 header）。已在 `apply_quick_setup_locked` 的 codex 分支被调用。

### 快速配置路径（`src-tauri/src/services/provider/mod.rs`）

- `apply_quick_setup(state, app_type, provider)` → `apply_quick_setup_locked`。
- codex 分支：`prepare_codex_provider_features_for_save` 被调用（做默认迁移），但**不接收显式生图/websocket 意图**。
- `Provider` 的 TOML 文本来自前端 `buildQuickSetupRequest`，当前只含 name/base_url/apiKey/modelId，不含生图/websocket 控制。

### 前端 v2（`src/v2`）

- `ProviderQuickSetupRequest { name, baseUrl, apiKey, modelId }`（`features/types.ts`）。
- `ProvidersPort.applyQuickSetupWithResult(request, app)`。
- `Page.tsx` 的 `ProviderPanel` 目前只有四个输入字段，无生图/websocket 开关。

### `requires_openai_auth` 现状

- 官方 provider 表在以下位置固定写 `true`：
  - `ensure_codex_feature_provider_table`（官方表 `requires_openai_auth = true`）。
  - `codex_official_provider_table`（`requires_openai_auth = true`）。
- 生图扩展写入（`set_managed_image_header`）**未联动修改** `requires_openai_auth`。

## 方案

### 1. 数据契约扩展

**前端** `src/v2/shared/features/types.ts`：

```ts
export interface ProviderQuickSetupRequest {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
  /** Codex 原生能力意图，仅 codex 目标生效 */
  codexFeatures?: {
    imageExtension?: boolean;
    websockets?: boolean;
  };
}
```

`quickSetup.ts` 的 `buildQuickSetupRequest` 透传 `codexFeatures`。

### 2. 后端快速配置接收意图

- 在 `apply_quick_setup` 的 codex 分支，把 `provider` 中携带的 `codexFeatures` 意图转成 `CodexProviderFeatureIntent`，在 `prepare_codex_provider_features_for_save` 之后、`normalize_provider_common_config_for_storage` 之前，调用 `patch_codex_provider_features`（或等效函数）把生图 header 与 `supports_websockets` 写入 TOML 草稿。
- 需要确认 `Provider` 结构体（`provider.rs`）是否有承载 `codexFeatures` 的字段；若无，需在 `Provider` 或快速设置专用请求结构上增加透传字段（建议在 `Provider.meta` 或新增字段，避免污染通用存储）。

### 3. 开启生图后 `requires_openai_auth = false`

在 `set_managed_image_header` 的调用点（`patch_codex_provider_features` 中，`intent.image_extension == Some(true)` 时）联动：

- 对官方 provider 表（`is_fixed_official_codex_provider` 为 true），当开启生图时写入 `requires_openai_auth = false`；关闭生图时恢复 `requires_openai_auth = true`。
- 注意：`generated_official_provider_table_is_safe_to_remove` 依赖 `requires_openai_auth == true` 判断「生成表可安全移除」；需同步审查该判断，避免开启生图后（值为 false）导致误删或误判。
- `codex_official_provider_table` 若被快速设置/代理路径复用，需同步加入生图参数以决定 `requires_openai_auth` 的取值。

### 4. 前端 UI 接入

- 在 `Page.tsx` 的 `ProviderPanel` 中，仅当 `app === "codex"` 时渲染两个开关（可复用 `CodexNativeCapabilities` 的视觉与文案，或新建轻量开关行）。
- 状态进入 `ProviderQuickSetupRequest.codexFeatures`，随 `submit` 提交。
- 复用现有 warning 文案（`CODEX_WEBSOCKET_NON_GPT_MODEL` / `CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED`）。

## 兼容性 / 回滚

- 新字段 `codexFeatures` 为可选，旧调用不受影响。
- 后端 `requires_openai_auth` 联动仅在显式生图意图时触发；默认迁移路径（`prepare_codex_provider_features_for_save`）保持现状，避免影响历史 provider。
- 若发现 `generated_official_provider_table_is_safe_to_remove` 与 `requires_openai_auth=false` 冲突，需引入额外标记（如仅比较 name/wire_api 且允许 `requires_openai_auth` 为 false 的受控场景）。

## 测试要点

- 前端：`tests/v2/pages/models/Page.test.tsx` 新增 codex 开关交互断言。
- 后端：`codex_config.rs` 与 `provider/mod.rs` 单测补充「开启生图 → header 写入 + requires_openai_auth=false；关闭 → 恢复 true」。
- 检查 `tests/lib/providersApi.codexFeatures.test.ts` 等既有测试不受影响。

## 待确认点（实施前需核对）

- `Provider` 结构体如何承载 codex 生图/websocket 意图（`src-tauri/src/provider.rs`）。
- 旧表单 `patchCodexProviderFeatures` 的 `requires_openai_auth` 是否也应联动（本次聚焦 v2，但需保持一致，避免两处行为漂移）。
