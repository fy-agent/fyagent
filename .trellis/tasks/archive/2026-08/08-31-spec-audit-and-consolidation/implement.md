# 实施步骤

## Stage A — 审计与任务基线

- [x] 清点 `.trellis/spec/**`：41 份、14,883 行。
- [x] 逐份阅读并记录结构、权威边界、重复与历史快照。
- [x] 检查索引覆盖、相对链接、TODO/TBD、重复标题和大段重复。
- [x] 建立 `research/spec-inventory.md`。

## Stage B — 导航与通用指南

- [x] 重写 backend/frontend/guides index。
- [x] 精简 code reuse 与 cross-layer thinking guide。
- [x] 精简 Frontend Reuse，保留 owner/placement/dependency/review 规则。

## Stage C — 长期规则与瞬时事实分离

- [x] 精简 Development Environment，改为权威文件驱动。
- [x] 精简 Upstream Sync，历史 SHA 交给 provenance ledger。
- [x] 移除 Development Hooks 的固定 Trellis 版本快照。
- [x] 清理少量日期化品牌迁移、handler 数量和工具版本复制。

## Stage D — V2 当前规则收敛

- [x] V2 Shell：将左侧导航、active-route-only 与 selection owner 直接写入正文。
- [x] V2 Agent/Models：移除旧 direct-jump/installed-only 条款，保留四分区配置壳。
- [x] V2 Skills/MCP：将 Agents 分区委托与权威 reread 规则合入正文。
- [x] V2 Prompts/Memory：将 Agents prompts 分区与 Memory shell 边界合入正文。

## Stage E — 三轮评审与验证

- [x] Round 1：结构、链接、索引、标记和 inventory 完整性。
- [x] Round 2：对照代码/配置/测试 owner 的语义复核与高风险负向抽查。
- [x] Round 3：完整 diff、格式、Trellis、docs/contracts 门禁。

## Stage F — 收尾

- [x] 完成完整 Trellis prearchive 门禁。
- [ ] 提交文档变更并推送 `dev/laiyongjie`。
- [ ] 完成 Trellis archive、journal 和 postarchive 验证。
- [ ] 创建目标为 `main` 的 PR，确认远端检查与合并条件后合并。
- [ ] 验证 `main` 合并结果与分支状态。
