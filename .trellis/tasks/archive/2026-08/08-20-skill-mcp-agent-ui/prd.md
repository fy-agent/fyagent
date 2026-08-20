# Skill/MCP 工具栏对齐与 Agent 目录介绍

## Goal

Skill/MCP 页头按钮与「已安装 / 发现」同一行；Skill 发现页保留两个按钮。六个非 Codex Agent 详情补上基于官方资料的实质性介绍。

## Requirements

- 把 view `FeatureTabs` 放进 `header.fy-feature-header`，右侧仍是两个主按钮。
- Skill 去掉 `width: auto` 对 view tabs 的全宽拉伸；分类 tabs 仍可在 toolbar 里独占换行。
- Skill 发现页保留「检查更新」「更多」（及有更新时的「更新全部」）。
- MCP 同样把页签与「导入现有 / 添加 MCP」放进同一 header。
- Agent 详情增加介绍区块；不渲染 catalog `description`；文案不得出现「使用说明」。Codex 不强制加介绍。
- V2 硬编码中文，不走 i18n。

## Acceptance Criteria

- [x] Skill 已安装与发现都能看到「检查更新」「更多」，且与页签同一行
- [x] MCP 「导入现有」「添加 MCP」与页签同一行
- [x] 六个非 Codex 详情可见多段介绍
- [x] 现有禁止 catalog description / 使用说明 / Hooks 面板的测试仍成立
