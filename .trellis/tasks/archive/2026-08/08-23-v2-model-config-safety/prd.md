# 修复 V2 模型配置安全与连通性

## Goal

修复 V2 Models 中 Codex 等模型配置场景的配置写入安全、真实模型连通性、保存状态和 API Key 输入控件问题，并审查共享实现，避免同类缺陷扩散到其他模型目标。配置保存必须最小化修改用户现有配置，不得以重建整个配置文件的方式覆盖无关字段。

## Background

- 用户提供的 Codex `config.toml` 含大量与模型 Provider 无关的顶层配置、features、MCP、plugins、desktop、projects 等内容；当前 V2 保存流程曾将其重建为一份明显缩减的配置，造成超范围覆盖风险。
- 用户截图显示 API Key 密码输入框的可见性按钮在部分状态下出现水平偏移。
- 用户截图和后续协议对照确认：Codex 的真实模型探测请求存在请求参数兼容性问题，当前实现会把原本可用的 Responses 请求变成失败。
- 用户截图显示保存成功后，旧的连通失败提示仍保留，且页头仍显示「待保存」。

## Requirements

### R1 — 最小范围配置写入

- Codex 保存不得重建或格式化整个 `config.toml`，不得删除、重排或改写与本次模型配置无关的顶层键、其他表、注释或用户自定义内容。
- 修改范围应收敛到本次操作明确拥有的键，例如当前 `model_provider` / `model` 以及目标 `[model_providers.<id>]` 内由 FyAgent 管理的字段；具体所有权必须由代码和官方 Codex 配置契约取证后在设计中明确。
- 已确认 Claude / Codex / Grok Build 的 V2 Quick Setup 当前都从最小 Provider 快照进入通用 Live 写入，其中均存在整文件替换同类风险；三者都属于本任务的保存安全范围。若能复用同一安全补丁边界则在共享所有者修复，不能安全共享时分别实现最小写入，但不得保留任一整文件重建路径。
- WorkBuddy / OpenCode 已有 read-modify-write 与单份备份机制；本任务只在真实测试证明有缺口时调整其存储实现，不为统一代码形态而重写已安全的旧模块。

### R2 — 写入前告知与单份备份

- 在实际修改用户配置前，UI 必须明确展示本次实际会修改的每一个物理配置文件路径；路径必须来自原生所有者，不得由 React 猜测拼接。
- 在实际修改前，必须明确告知会创建备份，并展示与每个实际写入文件对应的备份路径；Codex 若本次同时需要更新 `auth.json`，必须一并展示该文件及其备份。
- 每个实际会修改的已有配置文件仅保留一份滚动备份；后续保存更新这一份备份，不累积多代备份。
- 备份必须在目标文件写入前成功完成；备份失败时不得继续修改原文件。
- 首次创建、原文件不存在时不得伪造空备份；UI 仍需展示未来固定备份位置，并说明当前没有可备份前像。

### R3 — 真实模型连通性

- V2 Models 的「测试连通」必须按目标 Provider 的真实模型协议发送最小真实请求，而不是仅做 URL 可达性探测。
- Codex 自定义 Provider 的测试请求必须与 Codex `wire_api = "responses"` 语义一致，并使用当前表单中的 Base URL、API Key 和用户选择的模型 ID。
- Codex 探测请求不得继续发送已确认存在兼容性问题的额外限制字段。其余请求结构保持为受限的 Responses 语义，并在需要时按既有 Codex feature intent 投影受控请求头，而不是让前端提交任意 header。
- Grok Build Quick Setup 当前明确写入 `api_backend = "responses"`，因此其模型探测必须使用 `/v1/responses`，不得继续走 Chat Completions。
- 错误信息应保留足够诊断信息但不得泄露 API Key。
- 审查 Claude、Codex、Grok Build、OpenCode、WorkBuddy 等共享 `ModelConnectivityTest` 的目标适配；协议不同的目标不得被错误地强行共用同一请求格式。

### R4 — 保存状态一致性

- 保存成功后，应清除与保存前草稿绑定的旧连通性结果；旧的失败提示不得继续显示为当前保存配置的状态。
- 保存成功并重新基线化表单后，页头不得继续显示「待保存」。
- 只有当前表单与已保存基线存在真实差异时才显示「待保存」。
- 保存失败时不得错误清除草稿或把状态标为已保存。
- Provider Quick Setup、WorkBuddy、OpenCode 三类可写 Models 面板都必须采用同一“草稿版本 / 已提交版本”语义，不得继续用“字段非空即待保存”判断。

### R5 — API Key 输入控件稳定性

- API Key 的显示/隐藏按钮必须始终稳定地锚定在输入框末端，不因密码/明文切换、值变化、验证状态或重渲染发生偏移。
- 优先复用项目已有共享输入/密码控件；如现有共享实现存在缺陷，应修复共享所有者而不是在 Codex 页面做局部 CSS 补丁。
- 审查 V2 和仍被 V2 合法复用的旧前端共享组件，避免重复造轮子。

### R6 — 调研与复用约束

- 实现前必须核对项目现有组件、保存模块和测试，再核对 Codex / OpenAI 官方协议以及成熟开源实现。
- 有合适的项目内或成熟开源组件/模块可直接复用时优先复用；不得为了本任务无依据地自造一套平行实现。
- 后端旧模块以兼容和最小改动为优先，不做与验收无关的大范围重构。

### R7 — Trellis 知识沉淀

- 任务归档前按 Trellis 要求更新相关 `.trellis/spec/`，至少覆盖本次确认的配置最小写入、备份、真实模型探测和共享 UI 状态契约；仅记录经实现和验证确认的长期约束。

## Acceptance Criteria

- [ ] 给定包含大量无关 Codex 配置、注释和其他表的 fixture，保存模型 Provider 后仅允许 owned keys / owned provider table 发生预期变化，其余内容按设计声明保持稳定；不得出现整文件缩减重建，也不得无条件覆盖用户既有 `disable_response_storage`、其他 provider header 或非 FyAgent 配置。
- [ ] Claude fixture 的非 Quick Setup `settings.json` 顶层字段及其他 `env` 键保持；Grok Build fixture 的其他模型、MCP、权限/功能等无关 TOML 保持。切离再切回固定 V2 Quick Setup Provider 也不得重新触发整文件覆盖。
- [ ] 保存前 UI 可见地展示每一个实际目标配置文件路径与单份备份路径；已有文件备份失败时原配置不被修改；重复保存不累积多份备份；首次创建不生成虚假的空前像备份。
- [ ] WorkBuddy 继续保持 unknown JSON 字段、并发前像校验和其固定单份备份语义；OpenCode 继续保持无关根/provider JSON 字段和固定 `opencode.json.backup`，除非测试发现真实缺口才做窄修复。
- [ ] Codex probe 的本地 wire-level 回归测试证明：修复后的 Responses 请求不再携带已确认不兼容的额外限制字段，并能正确识别成功/失败流式响应；本任务不依赖真实外部 API 作为自动验收条件。
- [ ] Grok Build 探测的 wire test 明确请求 `/v1/responses`；Claude 仍为 Messages，WorkBuddy/OpenCode 仍为各自受支持的 OpenAI-compatible Chat 路径。
- [ ] Provider Quick Setup、WorkBuddy、OpenCode 保存成功后旧「连通测试失败」提示消失，页头「待保存」消失；随后再次进行用户编辑时「待保存」重新出现；保存进行中产生的新编辑不得被旧请求的成功结果错误清成已保存。
- [ ] API Key 显示/隐藏切换、输入、保存后清空/回填等状态下，末端按钮位置稳定且仍可访问。
- [ ] 共享模型页面中受同一保存、连通性或 secret-input 组件影响的目标均有回归覆盖；协议不同的目标有明确适配测试。
- [ ] 相关单元/组件/浏览器测试、Rust 测试和 Trellis/项目质量门禁通过。
- [ ] 相关 spec 已更新，任务完成后按 Trellis 流程归档。

## Out of Scope

- 不重构整个旧前端或全部 Provider 后端架构。
- 不迁移用户所有 Codex 配置到 FyAgent 自定义 schema。
- 不改变与本次模型 Provider 保存无关的 MCP、plugins、desktop、projects、memory、hooks 等用户配置。
- 不把任何真实凭据写入仓库、任务文档、测试 fixture、日志或提交记录。
