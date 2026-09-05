# 执行与验收

- [x] 检查起始分支/工作树、读取 Trellis 与两层 SPEC、全仓清点及固定阈值重复扫描。
- [x] 第一轮评审：依据实际调用方与成熟库选择四项修改，拒绝按行数拆文件。
- [x] 第二轮评审：核对抑制/事务/备份/凭证/可见性兼容性，完成 design。
- [x] 激活任务，执行相关基线检查。
- [x] 替换 SemVer 实现并加入反例测试。
- [x] 合并自动同步调度并通过组合根注入数据库通知；加入虚拟时间/隔离/数据库钩子测试。
- [x] 三个 MCP 适配器复用 JSON 文档拥有者；运行全部 MCP 回归。
- [x] 保存工作区共用编排并采用 Query 轮询；加入生命周期、并发、终态和密钥缓存测试。
- [x] 第三轮：检查整个 diff、架构边界、所有调用方与失败路径；重新扫描并记录真实差异。
- [x] 更新所属 SPEC；执行 typecheck、V2 lint/tests、Rust check/clippy/tests、renderer build 及完整 prearchive 检查；另完成 164 项浏览器回归。

收口顺序：已验证的工作提交 -> Trellis 归档提交 -> 会话记录提交；归档后无排除参数重跑 canonical contracts，并确认工作树干净、不推送。该步骤的最终完成证据为任务归档状态、Git 提交序列和交付时的实际状态，不提前标记完成。

## 命令

使用 `mise run` 的既有入口：`rust:test <filter>`、`test:unit -- tests/architecture`、`typecheck:v2`、`lint:v2`、`test:v2`、`build:renderer`。根 Vitest 明确排除 `tests/v2`，V2 必须使用独立的 `test:v2` 入口。最终运行：

```sh
mise run check:prearchive --exclude-active-task .trellis/tasks/09-05-architecture-debt-reuse
```

重复扫描命令和统计口径见 research/audit.md。失败须记录原始类别与修复；没有执行的原生平台验证不得标为通过。测试结果写入 review.md 后才能完成验收勾选。
