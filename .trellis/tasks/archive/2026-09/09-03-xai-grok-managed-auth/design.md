# Design

## Boundary

优先采用 Grok 官方 auth_provider_command/token broker 能力；不可用时使用官方 auth.json schema 的 read-modify-write、锁、原子替换和 generation reconciliation。

## Shared Invariants

- 不复制父任务定义的 OAuth、SecretRef、数据库、Proxy、Agent Auth、V2 Port 或 native projection owner。
- token、authorization code、cookie、原始 vendor 输出、绝对用户路径和自由命令不跨 renderer wire。
- 成功必须经过写后 readback；未知与部分成功保持显式状态。
- 新依赖必须先完成官方能力、许可证、维护、安全、平台、体积和现有依赖重复性评审。

## Compatibility and Rollback

- 保留现有已发布数据，迁移失败保持旧版本可运行或明确 blocked/recovery_required。
- 每个 mutation 有可恢复边界；无法证明回滚时禁止继续写入。
- 本子任务不得提前开放后续子任务尚未实现的能力。
