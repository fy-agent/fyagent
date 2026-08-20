# MCP 精选发现与安装 — 执行计划

## Ordered checklist

1. 新增 `mcpSecurity.ts`，改 `buildMcpSearchText` 与 MCP 普通详情展示；补 helpers / featurePages 脱敏测试。
2. 新增 `mcpLaunch.ts`，修正 `presets.ts` 的 Time 与 npx；更新 `tests/config/mcpPresets.test.ts`。
3. 实现 `catalog.ts`：11 条首发 builders、签名比较、分类枚举。
4. 抽出 `DEFAULT_NEW_APPS`；MCP Page 增加已安装/发现页签，已安装内容原样迁入。
5. 实现 `Discovery.tsx` 搜索/分类/卡片状态与一键安装。
6. 实现 `InstallDialog.tsx`：业务字段、password 遮罩、Agent 多选、关闭清状态、覆盖确认。
7. 为每个 builder 与冲突/默认分配写单测；补发现页 UI 测试。
8. 更新 `.trellis/spec/frontend/v2-skills-mcp.md`：搜索脱敏、npx 未知宿主、发现页边界。
9. 跑 V2 质量门。

## Validation

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check
git diff --check
```

至少覆盖：

- helpers：高德 URL、飞书 `-s`、env/headers 不进搜索。
- catalog builders：缺字段失败；Windows/macOS npx；Filesystem 空目录失败；钉钉不写 `ALL`。
- presets：Time=`uvx mcp-server-time`；npx 在 unknown 上不再消失。
- featurePages：已安装页签仍是默认；发现页安装走 upsert；同 ID 不静默覆盖；普通 UI 无 sentinel secret。

## Risky files

- `src/v2/pages/mcp/Page.tsx`：只加页签协调，避免重写编辑器。
- `src/v2/shared/features/helpers.ts`：搜索行为变化需同步 placeholder 与测试。
- `src/v2/shared/features/presets.ts`：影响「添加 MCP」模板。
- `.trellis/spec/frontend/v2-skills-mcp.md`：合同必须与实现一致。

## Rollback points

- Step 1 可单独合入（安全修复）。
- Step 4 之后若发现页不稳，可隐藏发现页签并保留脱敏。
- 不修改 Rust MCP schema，回滚不需要数据迁移。

## Follow-up before start

- 用户已授权规划完成后直接实施到完成，不再等待二次批准。
- 不改 Skills 及其他非 MCP 页面。
- 工作树若已有无关 Skills 改动，保持兼容，不覆盖。
