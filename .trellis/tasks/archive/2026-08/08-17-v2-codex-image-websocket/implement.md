# 实施计划 — Codex 一键配置接入生图与 WebSocket

## 分批提交计划

### 批次 1：后端生图联动 `requires_openai_auth`（可独立构建 + 单测）

1. `src-tauri/src/codex_config.rs`：
   - 在 `set_managed_image_header` 的调用点（`patch_codex_provider_features` 中 `intent.image_extension == Some(true)` 时），对官方 provider 表联动写入 `requires_openai_auth = false`；关闭时恢复 `true`。
   - 审查并修正 `generated_official_provider_table_is_safe_to_remove` / `remove_generated_official_provider_table`，确保 `requires_openai_auth = false`（开启生图）时不会误判可移除。
   - 审查 `codex_official_provider_table` 是否需要新增生图参数以决定 `requires_openai_auth` 取值。
2. 补充单测：开启生图 → header 写入 + `requires_openai_auth=false`；关闭 → 恢复 `true`。
3. 验证：`cargo test` 通过。

### 批次 2：后端快速配置路径接收生图/websocket 意图（可独立构建 + 单测）

1. 确认 `Provider` 结构体（`src-tauri/src/provider.rs`）如何承载 codex 生图/websocket 意图（新增字段或复用 meta）。
2. `src-tauri/src/services/provider/mod.rs`：
   - `apply_quick_setup_locked` 的 codex 分支：在 `prepare_codex_provider_features_for_save` 之后，若 provider 携带显式意图，调用 `patch_codex_provider_features` 把生图 header 与 `supports_websockets` 写入 TOML 草稿。
   - 透传 warning code（websocket 已有）。
3. 补充单测：快速设置携带生图/websocket 意图 → 最终 TOML 含对应内容。

### 批次 3：前端数据契约与 UI 开关（可独立构建 + 测试）

1. `src/v2/shared/features/types.ts`：`ProviderQuickSetupRequest` 增加可选 `codexFeatures?: { imageExtension?: boolean; websockets?: boolean }`。
2. `src/v2/pages/models/quickSetup.ts`：`buildQuickSetupRequest` 透传 `codexFeatures`。
3. `src/v2/pages/models/Page.tsx`：`ProviderPanel` 在 `app === "codex"` 时渲染两个开关，状态进入 `codexFeatures`，随 `submit` 提交。
4. 复用 websocket warning 文案。
5. 验证：前端 tsc + `tests/v2/pages/models/Page.test.tsx` 相关断言。

### 批次 4：端到端联调与回归（可选，合并进批次 3 后统一提交）

1. 核对旧 provider 表单（`CodexNativeCapabilities` / `useCodexProviderFeatures`）与 v2 路径行为一致，必要时同步 `requires_openai_auth` 联动。
2. 运行前后端全量测试。

## 验证命令

- 后端：`cargo test`（codex_config / provider 相关）
- 前端：`pnpm lint`、`pnpm tsc`、`tests/lib/providersApi.codexFeatures.test.ts`、`tests/v2/pages/models/Page.test.tsx`

## 回滚点

- 每个批次独立提交；生图联动逻辑与 UI 接入解耦，可分别回滚。
