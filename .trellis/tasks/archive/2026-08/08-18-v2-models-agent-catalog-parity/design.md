# Design

## Architecture and boundaries

V2 仍按现有端口分层：`pages → shared/features → shared/platform/{tauri,browser}`。Leftover React 只作行为参考。新增原生能力放在现有 `traework` / `opencode_config` / `skill` / `mcp` 服务旁，不新开通用 Provider 域给 WorkBuddy 或 TRAE。

Catalog v3 形状不变。Rust 更新：

- TRAE `display_name` = `TRAE Work CN`
- TRAE product URL = `https://www.trae.cn/sem-work`
- OpenCode `models.write` = `direct` + `dedicated_native_contract`
- TRAE `models.write` = `direct` + `dedicated_native_contract`（保存走本设计的 vscdb 投影，不再是 vendor_ui_required）
- 各 `description` 改成“支持 X；不支持 Y”句式，避免 “可通过 FyAgent …”

渲染层单独持有中文短标签，不把“支持”写进 Rust enum。

## Data flow

### Agent directory

```
get_agent_catalog → capability mode
  direct     → 徽章「支持」+ 跳转 /models|/skills|/mcp 或官网
  assisted   → 「需在应用中完成」+ 官网
  unsupported→ 默认折叠/不占主列表
  unverified → 「暂无法确认」
```

检测/启动保持 `null` 即未知。Qoder Hooks 区块仅在 QoderWork 选中时存在。

### TRAE models

```
UI draft (ids + connection, key in component memory)
  → validate_traework_model_config / fetch models
  → save_traework_models
      backup JSON blob
      clone Work-mode preset template
      upsert/delete custom rows in solo_work_lite + solo_work_remote
      HMAC revision
  → get_traework_model_ids (secret-free)
```

命令建议：

```text
get_traework_model_ids() -> { modelIds, revision, truncated }
fetch_traework_models({ request }) -> { models: [{id, ownedBy?}], truncated }
save_traework_models({ ...connection, selectedModelIds, removedModelIds, expectedRevision, overwriteToken? })
```

`request` 复用现有 TRAE URL/format 准入。保存必须在写入前完成一次成功的 validate（不必每次重跑长 probe，但首次添加需要可达结果或明确的 auth/model 拒绝以外的失败）。删除已有自定义 id 不要求新的网络 probe。

密钥：只出现在 mutation 参数。DTO/query/DOM 禁止 `ak`/`sk`。模型 id 与完整 key 碰撞则 fail closed。

路径：macOS `Application Support/TRAE SOLO CN/User/globalStorage/state.vscdb`；Windows 走 Explorer 用户配置目录等价路径，句柄规则对齐 WorkBuddy/Qoder。测试只用 `FYAGENT_TEST_HOME` 夹具库。

### OpenCode models

```text
get_opencode_model_snapshot() -> { providers: [{id, name, modelIds}], revision }
fetch_opencode_provider_models({ baseUrl, apiKey, allowNoApiKey }) -> { models, truncated }
save_opencode_models({ providerName, baseUrl, apiKey, selectedModelIds, removedModelIds, expectedRevision, overwriteToken? })
```

实现复用 `opencode_config` live provider 读写。V2 面板按 WorkBuddy 几何：一个当前第三方 provider + 模型芯片。不调用 Claude/Codex `applyQuickSetupWithResult`。

### Claude models

现有 `ProviderPanel` 增加 fetch（`fetch_models_for_config` 的 V2 端口）和芯片。保存仍是一条 reserved quick setup。`get_provider_summary` 可增加可选非密钥 `modelId`；若碰撞检测失败则整表 generic fail。

### Skills / MCP

```text
SkillTargetId += workbuddy     // serde "workbuddy"
schema 18: enabled_workbuddy INTEGER NOT NULL DEFAULT 0
copy dest: <trusted-home>/.workbuddy/skills

McpApps.workbuddy: bool        // default false, not AppType
live file: <trusted-home>/.workbuddy/.mcp.json   // mcpServers map
```

V2 `SKILL_TARGETS` / `MCP_TARGETS` 只列出 PRD R5。Gemini/Grok/Hermes 仍可被 leftover 与导入路径读写。Qoder/TRAE 继续拒绝 `toggleApp` 的 MCP 直接分配。

### Model icons

`src/v2/shared/assets/models/index.ts`：

```ts
resolveModelVendorIcon(modelId: string, ownedBy?: string | null): string
```

先 `ownedBy` 别名，再 model id 前缀（`gpt-`/`o1-`/`claude`/`deepseek`/`qwen`/`gemini`/`grok`/`kimi`/`minimax`/`mistral`/`llama`/`glm`/`doubao`/`seed-` 等）。资源是拷贝后的本地 SVG/PNG。`GroupedModelChips` 与 Claude/Codex 模型字段共用。

### QoderWork CN icon

`sips` 从 `QoderWork CN.app` 的 `icon.icns` 导出 256 PNG → `src/v2/shared/assets/agents/qoderwork.png`。`agentBrandById.qoderwork` 与 `skillTargetIconById.qoderwork` 改指向 PNG。删除或停止引用错误 SVG。

## Compatibility

- Catalog 测试夹具中的 displayName/URL/OpenCode models.write 必须同步。
- Skill schema 17 → 18：旧行六+二标志不变，新列 false。
- MCP JSON 缺 `workbuddy` 视为 false。
- TRAE vscdb 不存在或只有 preset：已有列表为空，允许首次保存创建自定义行。
- Browser adapter：新读写一律 native-only unavailable。

## Trade-offs

- TRAE 写入 vscdb 而不是虚构 `settings.json`：这是本机 SOLO CN 的真实存储。风险是字段多、版本易变。约束：只 clone preset 结构、只改自定义行、有 backup/revision、夹具测试覆盖。失败则不宣称保存。
- Skills/MCP 从 V2 隐藏 Gemini/Grok/Hermes：目录一致性优先于 leftover 全集暴露。后端标志保留，避免数据迁移破坏。
- Claude 仍一条 reserved Provider：避免在 V2 恢复通用 Provider CRUD。

## Rollback

- Catalog/文案/图标：回退对应资源与字符串。
- TRAE 保存：同目录 backup blob / 覆盖前副本。
- Skill 18：向前兼容默认 false；回退需保留列或再迁移，发布说明写明。
- WorkBuddy MCP：只触碰 `.mcp.json`，先写 backup。
