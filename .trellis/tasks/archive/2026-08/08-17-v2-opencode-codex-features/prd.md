# v2 支持 OpenCode 与 Codex 一键生图/WebSocket 配置

## Goal

在 FyAgent v2 前端页面体系中补齐两类能力：

1. **OpenCode 全维度支持**：让 OpenCode 作为一个受支持的应用，出现在 v2 的 agent 目录、模型配置、skill、MCP、提示词五个维度中，与已有的 Claude Code / Codex / QoderWork / TRAE Work 等应用对齐。
2. **Codex 一键配置接入生图与 WebSocket**：在 v2 模型配置页的 Codex 模块中，新增「启用内置生图扩展」与「启用 WebSocket 传输」两个开关，复用后端已有的 Codex 原生能力（`x-openai-actor-authorization` header 与 `supports_websockets`），并在开启生图扩展时令官方 provider 的 `requires_openai_auth = false`。

## Requirements

- 参考既有提交历史中「开启生图」「websocket」相关配置（`codex_config.rs` 中 `CODEX_IMAGE_EXTENSION_HEADER` / `CODEX_IMAGE_EXTENSION_VALUE` / `supports_websockets`），将能力暴露到 v2 模型配置页 Codex 模块。
- 开启生图扩展后，官方 Codex provider 表的 `requires_openai_auth` 值应为 `false`。
- 分批次提交（分阶段、分功能点提交，不一次性大提交）。

## Acceptance Criteria

- [ ] OpenCode 出现在 agent 目录页，且其条目可正常交互。
- [ ] OpenCode 出现在模型配置页，且提供合理的快速配置或引导。
- [ ] OpenCode 在 skill / MCP / 提示词页面中可被正常选中、配置、同步。
- [ ] v2 模型配置页 Codex 模块出现「启用内置生图扩展」与「启用 WebSocket 传输」开关，且功能真正生效。
- [ ] 开启生图扩展后，官方 Codex provider 的 `requires_openai_auth` 为 `false`。
- [ ] 分批次提交，每批可独立构建/测试通过。

## Notes

- 本任务为父任务，拆分为两个可独立验证的子任务：
  - `08-17-v2-opencode-support`（OpenCode 全维度支持）
  - `08-17-v2-codex-image-websocket`（Codex 生图/WebSocket 一键配置）
- 两个子任务各自产出 prd/design/implement，独立实施、独立检查、独立归档。
- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
