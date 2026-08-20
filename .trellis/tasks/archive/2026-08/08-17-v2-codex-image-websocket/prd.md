# Codex 一键配置接入生图与 WebSocket 能力

## Goal

在 v2 模型配置页的 Codex 模块（`src/v2/pages/models/Page.tsx` 的 `ProviderPanel`）中新增「启用内置生图扩展」与「启用 WebSocket 传输」两个开关，复用后端已有的 Codex 原生能力，并在开启生图扩展时令官方 provider 的 `requires_openai_auth = false`。

## Requirements

### 现状（后端已具备的能力）

- `src-tauri/src/codex_config.rs`：
  - 生图扩展：`CODEX_IMAGE_EXTENSION_HEADER = "x-openai-actor-authorization"`，`CODEX_IMAGE_EXTENSION_VALUE = "local-image-extension"`；`set_managed_image_header` 负责写入/删除 `http_headers`。
  - WebSocket：`supports_websockets` 字段的读取（`websocket_state`）与写入（`patch_codex_provider_features` 中 `provider_table.insert("supports_websockets", true)`）。
  - `analyze_codex_provider_features` / `patch_codex_provider_features` 是旧 provider 表单用的分析/补丁 API。
  - 官方 provider 表当前固定写 `requires_openai_auth = true`（`build_*` 与 `patch` 路径）。
- `src-tauri/src/services/provider/mod.rs`：
  - `apply_quick_setup` 已支持 codex 快速设置写入 `supports_websockets = true` 并返回 warning code（`CODEX_WEBSOCKET_NON_GPT_MODEL` / `CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED`），但**未处理生图扩展 header**。
- 前端旧 provider 表单已有 `CodexNativeCapabilities` 组件与 `useCodexProviderFeatures` hook，但 v2 模型配置页未复用。

### 需要补齐

1. **v2 模型配置页 Codex 模块接入两个开关**：
   - 在 `ProviderPanel`（`src/v2/pages/models/Page.tsx`）中，为 codex 目标新增「启用内置生图扩展」「启用 WebSocket 传输」两个开关。
   - 快速配置请求结构需携带这两个开关的意图，并传递给后端 `applyQuickSetup`。
2. **后端快速配置路径接入生图/WebSocket**：
   - `apply_quick_setup`（或新增的 codex 特化路径）根据开关意图，在生成的 TOML 中写入生图 header 与 `supports_websockets`。
3. **开启生图后 `requires_openai_auth = false`**：
   - 当生图扩展开启时，官方 Codex provider 表（`[model_providers.custom]` 或官方表）的 `requires_openai_auth` 应写为 `false`（生图扩展走本地 `x-openai-actor-authorization` header，不需要 OpenAI 官方登录授权）。
   - 关闭生图时恢复 `requires_openai_auth = true`。

## Acceptance Criteria

- [ ] v2 模型配置页 Codex 模块出现「启用内置生图扩展」与「启用 WebSocket 传输」两个开关。
- [ ] 开启生图扩展后，生成的 Codex TOML 中含 `http_headers = { "x-openai-actor-authorization" = "local-image-extension" }`。
- [ ] 开启生图扩展后，官方 provider 的 `requires_openai_auth` 为 `false`；关闭后为 `true`。
- [ ] 开启 WebSocket 后，生成的 TOML 含 `supports_websockets = true`，并复用已有 warning 提示。
- [ ] 快速配置保存后功能真实生效，与旧表单行为一致。
- [ ] 无类型错误、无 lint 错误，相关前后端测试通过。

## Notes

- 需查阅之前提交历史中「开启生图」「websocket」相关配置作为参考实现依据（`codex_config.rs`）。
- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
