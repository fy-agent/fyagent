# Implement

1. 模型页去掉两个官方设置按钮，更新 Page / Trae / browser 测试。
2. `FeatureList` 改为纵向 flex，补一条样式/组件测试证明 lens 不是 grid 项。
3. Native MCP：`McpTargetId`、`McpApps`、schema v19、`mcp/qoderwork.rs`、`mcp/traework.rs`、service sync/import/remove、deeplink 允许这两个 id。
4. Catalog `mcp.write` 改为 direct dedicated native contract；补 adapter 单测（skip / write / backup / import）。
5. V2 types、icons、`DEFAULT_NEW_APPS`、helpers 标签、featurePorts / AssignmentPanel / MCP page 测试改为六目标。
6. Merge PR #111，按本分支共享化解冲突，跑 prompts/memory/skills 相关测试。
7. 归档 `08-18-prompt-memory-frontend-replan`，关闭 PR #111。
8. 更新 `v2-skills-mcp.md`、`v2-agent-models.md`、`external-agent-p0.md`、frontend index。

## Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
git diff --check
```

按需：`mise run test:v2:browser` 覆盖模型页按钮消失、MCP 六开关、列表不重叠。
